//! Connection-time DNS policy for sender-controlled unsubscribe hosts.
use std::net::IpAddr;

use reqwest::dns::{Addrs, Name, Resolve, Resolving};

struct SystemResolver;

impl Resolve for SystemResolver {
    fn resolve(&self, name: Name) -> Resolving {
        Box::pin(async move {
            let addresses = tokio::net::lookup_host((name.as_str(), 0)).await?;
            Ok(Box::new(addresses.collect::<Vec<_>>().into_iter()) as Addrs)
        })
    }
}

pub(super) fn resolver(allow_loopback_for_tests: bool) -> impl Resolve {
    PublicResolver {
        inner: SystemResolver,
        allow_loopback_for_tests,
    }
}

pub(super) struct PublicResolver<R> {
    pub(super) inner: R,
    pub(super) allow_loopback_for_tests: bool,
}

impl<R: Resolve> Resolve for PublicResolver<R> {
    fn resolve(&self, name: Name) -> Resolving {
        let allow_loopback = self.allow_loopback_for_tests && name.as_str() == "localhost";
        let pending = self.inner.resolve(name);
        Box::pin(async move {
            let addresses: Vec<_> = pending.await?.collect();
            // INVARIANT: Validate the exact addresses returned to the connector, with no second
            // lookup. Reject mixed answers too, so fallback cannot connect to a private address.
            if addresses.is_empty()
                || addresses.iter().any(|address| {
                    !public_address(address.ip()) && !(allow_loopback && address.ip().is_loopback())
                })
            {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::PermissionDenied,
                    "unsubscribe DNS returned a non-public or empty address set",
                )
                .into());
            }
            Ok(Box::new(addresses.into_iter()) as Addrs)
        })
    }
}

fn public_address(address: IpAddr) -> bool {
    match address {
        IpAddr::V4(ip) => {
            let [a, b, c, _] = ip.octets();
            // INVARIANT: Conservatively exclude special-use IPv4 ranges, including shared,
            // benchmarking, documentation, deprecated relay, multicast and reserved space.
            !matches!(a, 0 | 10 | 127 | 224..=255)
                && !(a == 100 && (64..=127).contains(&b))
                && !(a == 169 && b == 254)
                && !(a == 172 && (16..=31).contains(&b))
                && !(a == 192
                    && (b == 168 || (b == 0 && matches!(c, 0 | 2)) || (b == 88 && c == 99)))
                && !(a == 198 && (b == 18 || b == 19 || (b == 51 && c == 100)))
                && !(a == 203 && b == 0 && c == 113)
        }
        IpAddr::V6(ip) => {
            let [a, b, ..] = ip.segments();
            // INVARIANT: Allow ordinary global unicast only. Exclude special protocols,
            // documentation and 6to4; mapped/NAT64/local addresses stay outside this allowlist.
            (a & 0xe000) == 0x2000
                && !(a == 0x2001 && (b < 0x0200 || b == 0x0db8))
                && a != 0x2002
                && !(a == 0x3fff && b < 0x1000)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn excludes_local_special_and_transition_addresses() {
        for address in [
            "0.0.0.0",
            "10.0.0.1",
            "100.64.0.1",
            "100.127.255.255",
            "127.0.0.1",
            "169.254.169.254",
            "172.16.0.1",
            "172.31.255.255",
            "192.168.1.1",
            "192.0.0.1",
            "192.0.2.1",
            "192.88.99.1",
            "198.18.0.1",
            "198.19.255.255",
            "198.51.100.1",
            "203.0.113.1",
            "224.0.0.1",
            "255.255.255.255",
            "::",
            "::1",
            "::ffff:127.0.0.1",
            "::ffff:8.8.8.8",
            "64:ff9b::7f00:1",
            "64:ff9b:1::1",
            "100::1",
            "fc00::1",
            "fe80::1",
            "ff02::1",
            "2001::1",
            "2001:2::1",
            "2001:db8::1",
            "2002:7f00:1::1",
            "3fff::1",
        ] {
            assert!(!public_address(address.parse().unwrap()), "{address}");
        }
        for address in [
            "8.8.8.8",
            "1.1.1.1",
            "2606:4700:4700::1111",
            "2001:4860:4860::8888",
        ] {
            assert!(public_address(address.parse().unwrap()), "{address}");
        }
    }
}
