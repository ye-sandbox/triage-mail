//! MX-based provider hints. Only a typed domain's MX names are inspected; nothing is resolved further.
use super::{normalize_host, DiscoveredProvider, DiscoverySource, OAuthEndpoints};
use crate::AuthMethod;

pub(super) const MAX_MX_RECORDS: usize = 10;

/// Normalizes, de-duplicates and caps raw exchange names; invalid names are dropped.
pub(super) fn sanitize(raw: impl IntoIterator<Item = String>) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for name in raw {
        if out.len() >= MAX_MX_RECORDS {
            break;
        }
        if let Ok(host) = normalize_host(name.trim_end_matches('.')) {
            if !out.contains(&host) {
                out.push(host);
            }
        }
    }
    out
}

pub(super) fn classify(domain: &str, exchanges: &[String]) -> Option<DiscoveredProvider> {
    let google = |h: &str| {
        h.ends_with(".l.google.com") || h.ends_with(".googlemail.com") || h.starts_with("aspmx.")
    };
    let microsoft = |h: &str| h.ends_with(".mail.protection.outlook.com");
    // INVARIANT: Require every exchange to agree, so a mixed/forwarding setup is not mislabeled.
    if exchanges.is_empty() {
        return None;
    }
    if exchanges.iter().all(|h| google(h)) {
        Some(build(domain, "Google Workspace", "imap.gmail.com", false))
    } else if exchanges.iter().all(|h| microsoft(h)) {
        Some(build(
            domain,
            "Microsoft 365",
            "outlook.office365.com",
            true,
        ))
    } else {
        None
    }
}

fn build(domain: &str, name: &str, host: &str, microsoft: bool) -> DiscoveredProvider {
    let oauth = if microsoft {
        OAuthEndpoints {
            authorization_endpoint:
                "https://login.microsoftonline.com/common/oauth2/v2.0/authorize".into(),
            token_endpoint: "https://login.microsoftonline.com/common/oauth2/v2.0/token".into(),
            scopes: "offline_access https://outlook.office.com/IMAP.AccessAsUser.All".into(),
        }
    } else {
        OAuthEndpoints {
            authorization_endpoint: "https://accounts.google.com/o/oauth2/v2/auth".into(),
            token_endpoint: "https://oauth2.googleapis.com/token".into(),
            scopes: "https://mail.google.com/".into(),
        }
    };
    DiscoveredProvider {
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
        oauth_endpoints: Some(oauth),
        help_url: (!microsoft).then(|| "https://myaccount.google.com/apppasswords".to_owned()),
        source: DiscoverySource::Mx,
    }
}
