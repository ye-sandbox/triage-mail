use super::{DiscoveredProvider, DiscoverySource, OAuthEndpoints};
use crate::AuthMethod;

pub(super) fn lookup(domain: &str) -> Option<DiscoveredProvider> {
    let (name, host, help) = match domain {
        "gmail.com" | "googlemail.com" => (
            "Gmail",
            "imap.gmail.com",
            Some("https://myaccount.google.com/apppasswords"),
        ),
        "outlook.com" | "hotmail.com" | "hotmail.com.br" | "outlook.com.br" | "live.com"
        | "live.com.br" | "msn.com" => ("Outlook", "outlook.office365.com", None),
        "yahoo.com" | "yahoo.com.br" | "ymail.com" => ("Yahoo", "imap.mail.yahoo.com", None),
        "icloud.com" | "me.com" | "mac.com" => ("iCloud", "imap.mail.me.com", None),
        "fastmail.com" | "fastmail.fm" => ("Fastmail", "imap.fastmail.com", None),
        "zoho.com" => ("Zoho", "imap.zoho.com", None),
        "uol.com.br" => ("UOL", "imap.uol.com.br", None),
        "bol.com.br" => ("BOL", "imap.bol.com.br", None),
        "terra.com.br" => ("Terra", "imap.terra.com.br", None),
        _ => return None,
    };
    let microsoft = name == "Outlook";
    Some(DiscoveredProvider {
        domain: domain.into(),
        provider_name: name.into(),
        imap_host: Some(host.into()),
        imap_port: 993,
        use_ssl: true,
        recommended_auth: if microsoft {
            AuthMethod::OAuth2PublicPkce
        } else {
            AuthMethod::AppPassword
        },
        oauth_endpoints: microsoft.then(|| OAuthEndpoints {
            authorization_endpoint:
                "https://login.microsoftonline.com/common/oauth2/v2.0/authorize".into(),
            token_endpoint: "https://login.microsoftonline.com/common/oauth2/v2.0/token".into(),
            scopes: "offline_access https://outlook.office.com/IMAP.AccessAsUser.All".into(),
        }),
        help_url: help.map(str::to_owned),
        source: DiscoverySource::Preset,
    })
}
