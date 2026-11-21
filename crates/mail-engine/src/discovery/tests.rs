use super::*;
use std::sync::atomic::{AtomicUsize, Ordering};

const CONFIG: &str = r#"<?xml version="1.0"?>
<clientConfig version="1.1"><emailProvider id="example.com">
<incomingServer type="pop3"><hostname>pop.example.com</hostname><port>995</port><socketType>SSL</socketType></incomingServer>
<incomingServer type="imap"><hostname>imap.%EMAILDOMAIN%</hostname><port>993</port><socketType>SSL</socketType><authentication>password-cleartext</authentication></incomingServer>
</emailProvider></clientConfig>"#;

struct Fixture {
    calls: AtomicUsize,
    slow: bool,
    bytes: Vec<u8>,
    mx: Vec<String>,
}
impl Default for Fixture {
    fn default() -> Self {
        Self {
            calls: AtomicUsize::new(0),
            slow: false,
            bytes: CONFIG.as_bytes().to_vec(),
            mx: Vec::new(),
        }
    }
}
#[async_trait]
impl DiscoveryTransport for Fixture {
    async fn ispdb(&self, domain: &str) -> Result<Vec<u8>, DiscoveryError> {
        assert_eq!(domain, "example.com");
        self.calls.fetch_add(1, Ordering::SeqCst);
        if self.slow {
            std::future::pending::<()>().await;
        }
        Ok(self.bytes.clone())
    }
    async fn probe(&self, host: &str) -> Result<bool, DiscoveryError> {
        assert_eq!(host, "imap.example.com");
        self.calls.fetch_add(1, Ordering::SeqCst);
        if self.slow {
            std::future::pending::<()>().await;
        }
        Ok(true)
    }
    async fn mx(&self, domain: &str) -> Result<Vec<String>, DiscoveryError> {
        assert_eq!(domain, "example.com");
        self.calls.fetch_add(1, Ordering::SeqCst);
        if self.slow {
            std::future::pending::<()>().await;
        }
        Ok(self.mx.clone())
    }
}

#[test]
fn parses_single_mailbox_without_header_or_host_injection() {
    for address in [
        "reader@example.com",
        "Reader <reader@EXAMPLE.COM>",
        "\"Reader, One\" <reader@example.com>",
        "\"local part\"@example.com",
    ] {
        assert_eq!(email_domain(address), Ok("example.com".into()), "{address}");
    }
    for address in [
        "",
        "reader",
        "@example.com",
        "a@@example.com",
        "a@localhost",
        "a@[127.0.0.1]",
        "a@127.0.0.1",
        "a@example.com,b@example.org",
        "team: a@example.com;",
        "a@example.com\r\nBcc: b@example.org",
        "a@bad_domain.example.com",
        "a@-bad.example.com",
        "a@example.com/path",
    ] {
        assert_eq!(
            email_domain(address),
            Err(DiscoveryError::InvalidInput),
            "{address}"
        );
    }
}

#[test]
fn presets_cover_exact_aliases_without_suffix_matching() {
    for (domain, provider) in [
        ("gmail.com", "Gmail"),
        ("outlook.com.br", "Outlook"),
        ("hotmail.com", "Outlook"),
        ("yahoo.com.br", "Yahoo"),
        ("me.com", "iCloud"),
        ("fastmail.fm", "Fastmail"),
        ("zoho.com", "Zoho"),
        ("uol.com.br", "UOL"),
        ("bol.com.br", "BOL"),
        ("terra.com.br", "Terra"),
    ] {
        let preset = presets::lookup(domain).unwrap();
        assert_eq!(preset.provider_name, provider);
        assert!(preset.use_ssl);
        assert_eq!(preset.imap_port, 993);
    }
    let microsoft = presets::lookup("hotmail.com").unwrap();
    assert_eq!(microsoft.recommended_auth, AuthMethod::OAuth2PublicPkce);
    assert!(microsoft.oauth_endpoints.is_some());
    let gmail = presets::lookup("gmail.com").unwrap();
    assert_eq!(gmail.recommended_auth, AuthMethod::AppPassword);
    assert_eq!(
        gmail
            .oauth_endpoints
            .as_ref()
            .map(|e| e.authorization_endpoint.as_str()),
        Some("https://accounts.google.com/o/oauth2/v2/auth")
    );
    assert!(presets::lookup("gmail.com.example.com").is_none());
    assert!(presets::lookup("example.com").is_none());
}

#[tokio::test]
async fn offline_has_zero_network_and_online_receives_only_domain() {
    let transport = Fixture::default();
    let local = discover("Reader <reader@example.com>", false, &transport)
        .await
        .unwrap();
    assert_eq!(local.source, DiscoverySource::Unresolved);
    assert_eq!(local.imap_host, None);
    assert_eq!(transport.calls.load(Ordering::SeqCst), 0);
    let online = discover("reader@example.com", true, &transport)
        .await
        .unwrap();
    assert_eq!(online.source, DiscoverySource::Ispdb);
    assert_eq!(online.imap_host.as_deref(), Some("imap.example.com"));
    assert!(online.oauth_endpoints.is_none());
    assert!(online.help_url.is_none());
    assert_eq!(transport.calls.load(Ordering::SeqCst), 1);
    assert!(probe_confirmed_host("imap.example.com", &transport)
        .await
        .unwrap());
    assert_eq!(transport.calls.load(Ordering::SeqCst), 2);
    assert_eq!(
        probe_confirmed_host("127.0.0.1", &transport).await,
        Err(DiscoveryError::InvalidInput)
    );
}

#[tokio::test]
async fn invalid_and_oversized_configuration_fail_closed_without_probe() {
    for bytes in [vec![0; MAX_CONFIG_BYTES + 1], b"malformed".to_vec()] {
        let transport = Fixture {
            bytes,
            ..Default::default()
        };
        assert_eq!(
            discover("reader@example.com", true, &transport)
                .await
                .unwrap()
                .source,
            DiscoverySource::Unresolved
        );
        assert_eq!(transport.calls.load(Ordering::SeqCst), 1);
    }
}

#[test]
fn xml_rejects_unsafe_and_malformed_endpoints() {
    for xml in [
        CONFIG.replace("SSL", "STARTTLS"),
        CONFIG.replace("993", "0"),
        CONFIG.replace("993", "65536"),
        CONFIG.replace("imap.%EMAILDOMAIN%", "127.0.0.1"),
        CONFIG.replace("imap.%EMAILDOMAIN%", "%EMAILADDRESS%"),
        CONFIG.replace("imap.%EMAILDOMAIN%", "https://imap.example.com"),
        format!("<!DOCTYPE test [<!ENTITY x SYSTEM 'file:///etc/passwd'>]>{CONFIG}"),
        format!("{CONFIG}<extra/>"),
        CONFIG.replace("</clientConfig>", ""),
        CONFIG.replace("imap.%EMAILDOMAIN%", "&unknown;"),
    ] {
        assert!(xml::parse("example.com", xml.as_bytes()).is_none());
    }
}

#[tokio::test(start_paused = true)]
async fn lookup_and_confirmed_probe_have_independent_three_second_deadlines() {
    let transport = Fixture {
        slow: true,
        ..Default::default()
    };
    let before = tokio::time::Instant::now();
    assert_eq!(
        discover("reader@example.com", true, &transport)
            .await
            .unwrap()
            .source,
        DiscoverySource::Unresolved
    );
    assert_eq!(before.elapsed(), DISCOVERY_TIMEOUT);
    let before = tokio::time::Instant::now();
    assert_eq!(
        probe_confirmed_host("imap.example.com", &transport).await,
        Err(DiscoveryError::Unavailable)
    );
    assert_eq!(before.elapsed(), DISCOVERY_TIMEOUT);
}

#[test]
fn xml_accepts_safe_references_but_rejects_excessive_nesting_and_trailing_content() {
    let safe = CONFIG
        .replace(
            "<emailProvider id=\"example.com\">",
            "<emailProvider id=\"example.com\"><displayName>Example &amp; Mail</displayName>",
        )
        .replace("imap.%EMAILDOMAIN%", "im&#97;p.%EMAILDOMAIN%");
    assert_eq!(
        xml::parse("example.com", safe.as_bytes())
            .unwrap()
            .imap_host
            .as_deref(),
        Some("imap.example.com")
    );
    let deep = format!(
        "<clientConfig>{}{}{}",
        "<a>".repeat(17),
        "</a>".repeat(17),
        "</clientConfig>"
    );
    assert!(xml::parse("example.com", deep.as_bytes()).is_none());
    assert!(xml::parse(
        "example.com",
        format!("{CONFIG}<![CDATA[extra]]>").as_bytes()
    )
    .is_none());
}

fn mx_fixture(names: &[&str]) -> Fixture {
    Fixture {
        mx: names.iter().map(|n| (*n).to_owned()).collect(),
        ..Fixture::default()
    }
}

#[tokio::test]
async fn mx_maps_google_and_microsoft_only() {
    let g = mx_fixture(&["aspmx.l.google.com.", "alt1.aspmx.l.google.com."]);
    let p = detect_mx("a@example.com", &g).await.unwrap();
    assert_eq!(p.source, DiscoverySource::Mx);
    assert_eq!(p.imap_host.as_deref(), Some("imap.gmail.com"));
    assert_eq!(p.recommended_auth, AuthMethod::AppPassword);
    let m = mx_fixture(&["example-com.mail.protection.outlook.com"]);
    let p = detect_mx("a@example.com", &m).await.unwrap();
    assert_eq!(p.imap_host.as_deref(), Some("outlook.office365.com"));
    assert_eq!(p.recommended_auth, AuthMethod::OAuth2PublicPkce);
}

#[tokio::test]
async fn mx_unknown_empty_mixed_and_invalid_are_unresolved() {
    for names in [
        vec!["mail.example.net"],
        vec![],
        vec!["aspmx.l.google.com", "mail.example.net"],
        vec!["127.0.0.1", "localhost", ""],
    ] {
        let p = detect_mx("a@example.com", &mx_fixture(&names))
            .await
            .unwrap();
        assert_eq!(p.source, DiscoverySource::Unresolved, "{names:?}");
    }
}

#[tokio::test]
async fn mx_caps_records_and_discover_stays_offline() {
    let mut many = vec!["mail.example.net".to_owned(); 5];
    many.extend((0..30).map(|i| format!("m{i}.l.google.com")));
    assert_eq!(mx::sanitize(many).len(), mx::MAX_MX_RECORDS);
    let f = mx_fixture(&["aspmx.l.google.com"]);
    discover("a@example.com", false, &f).await.unwrap();
    assert_eq!(f.calls.load(Ordering::SeqCst), 0);
}

#[tokio::test(start_paused = true)]
async fn mx_times_out_to_unresolved() {
    let f = Fixture {
        slow: true,
        ..mx_fixture(&[])
    };
    let p = detect_mx("a@example.com", &f).await.unwrap();
    assert_eq!(p.source, DiscoverySource::Unresolved);
}
