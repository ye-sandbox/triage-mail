use super::{DiscoveredProvider, DiscoverySource, OAuthEndpoints};
use crate::AuthMethod;

pub(super) fn lookup(domain: &str) -> Option<DiscoveredProvider> {
    let (name, host, help) = match domain {
        // INVARIANT: Hosts/ports below are verified against provider documentation; unverified
        // providers (e.g. globo.com, ig.com.br) are omitted instead of guessed. Mirrored by
        // `preset_domains.json` (parity-tested here and in `presets.ts`).
        "gmail.com" | "googlemail.com" => (
            "Gmail",
            "imap.gmail.com",
            Some("https://myaccount.google.com/apppasswords"),
        ),
        "outlook.com" | "hotmail.com" | "hotmail.com.br" | "outlook.com.br" | "live.com"
        | "live.com.br" | "msn.com" | "outlook.fr" | "outlook.de" | "outlook.es" | "outlook.it"
        | "outlook.com.ar" | "hotmail.co.uk" | "hotmail.fr" | "hotmail.de" | "hotmail.es"
        | "hotmail.it" | "live.co.uk" | "live.fr" | "live.de" | "live.it" => {
            ("Outlook", "outlook.office365.com", None)
        }
        "yahoo.com" | "yahoo.com.br" | "ymail.com" => ("Yahoo", "imap.mail.yahoo.com", None),
        "icloud.com" | "me.com" | "mac.com" => ("iCloud", "imap.mail.me.com", None),
        "fastmail.com" | "fastmail.fm" => ("Fastmail", "imap.fastmail.com", None),
        "zoho.com" => ("Zoho", "imap.zoho.com", None),
        "zoho.eu" => ("Zoho EU", "imap.zoho.eu", None),
        "gmx.com" | "gmx.net" | "gmx.de" | "gmx.at" | "gmx.ch" => ("GMX", "imap.gmx.net", None),
        "mail.com" => ("Mail.com", "imap.mail.com", None),
        "aol.com" => ("AOL", "imap.aol.com", None),
        "yandex.com" | "yandex.ru" => ("Yandex", "imap.yandex.com", None),
        "proton.me" | "protonmail.com" | "pm.me" => ("Proton", "127.0.0.1", None),
        "uol.com.br" => ("UOL", "imap.uol.com.br", None),
        "bol.com.br" => ("BOL", "imap.bol.com.br", None),
        "terra.com.br" => ("Terra", "imap.terra.com.br", None),
        _ => return None,
    };
    let proton = name == "Proton";
    let microsoft = name == "Outlook";
    let google = name == "Gmail";
    Some(DiscoveredProvider {
        domain: domain.into(),
        provider_name: name.into(),
        imap_host: Some(host.into()),
        imap_port: if proton { 1143 } else { 993 },
        use_ssl: true,
        recommended_auth: if proton {
            AuthMethod::ProtonBridgeLocal
        } else if microsoft {
            AuthMethod::OAuth2PublicPkce
        } else {
            AuthMethod::AppPassword
        },
        oauth_endpoints: if microsoft {
            Some(OAuthEndpoints {
                authorization_endpoint:
                    "https://login.microsoftonline.com/common/oauth2/v2.0/authorize".into(),
                token_endpoint: "https://login.microsoftonline.com/common/oauth2/v2.0/token".into(),
                scopes: "offline_access https://outlook.office.com/IMAP.AccessAsUser.All".into(),
            })
        } else if google {
            Some(OAuthEndpoints {
                authorization_endpoint: "https://accounts.google.com/o/oauth2/v2/auth".into(),
                token_endpoint: "https://oauth2.googleapis.com/token".into(),
                scopes: "https://mail.google.com/".into(),
            })
        } else {
            None
        },
        help_url: help.map(str::to_owned),
        source: DiscoverySource::Preset,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(serde::Deserialize)]
    struct Entry {
        domain: String,
        name: String,
        host: String,
        port: u16,
        auth: AuthMethod,
    }

    #[test]
    fn lookup_matches_shared_fixture() {
        let entries: Vec<Entry> =
            serde_json::from_str(include_str!("preset_domains.json")).expect("fixture");
        for e in &entries {
            let found = lookup(&e.domain).unwrap_or_else(|| panic!("missing {}", e.domain));
            assert_eq!(found.provider_name, e.name, "{}", e.domain);
            assert_eq!(
                found.imap_host.as_deref(),
                Some(e.host.as_str()),
                "{}",
                e.domain
            );
            assert_eq!(found.imap_port, e.port, "{}", e.domain);
            assert_eq!(found.recommended_auth, e.auth, "{}", e.domain);
        }
        assert!(lookup("globo.com").is_none() && lookup("example.com").is_none());
    }
}
