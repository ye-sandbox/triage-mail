//! Public destination address policy; no test-only bypass.
use std::net::IpAddr;
pub(crate) fn public_address(address: IpAddr) -> bool {
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
