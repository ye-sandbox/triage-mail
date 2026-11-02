use quick_xml::{events::Event, Reader};

use super::{normalize_host, DiscoveredProvider, DiscoverySource, MAX_CONFIG_BYTES};

#[derive(Default)]
struct Server {
    imap: bool,
    hostname: String,
    port: String,
    socket_type: String,
}

impl Server {
    fn provider(&self, domain: &str) -> Option<DiscoveredProvider> {
        if !self.imap || self.socket_type.trim() != "SSL" {
            return None;
        }
        let host = self.hostname.trim().replace("%EMAILDOMAIN%", domain);
        let host = normalize_host(&host).ok()?;
        let port: u16 = self.port.trim().parse().ok()?;
        if port == 0 {
            return None;
        }
        let mut result = DiscoveredProvider::unresolved(domain.into());
        result.imap_host = Some(host);
        result.imap_port = port;
        result.source = DiscoverySource::Ispdb;
        // INVARIANT: XML cannot establish OAuth trust, launch links, select plaintext/STARTTLS,
        // or initiate connections. The user reviews the proposed implicit-TLS endpoint.
        Some(result)
    }
}

pub(super) fn parse(domain: &str, bytes: &[u8]) -> Option<DiscoveredProvider> {
    if bytes.len() > MAX_CONFIG_BYTES {
        return None;
    }
    let mut reader = Reader::from_reader(bytes);
    reader.config_mut().expand_empty_elements = true;
    let mut path: Vec<String> = Vec::new();
    let mut server = Server::default();
    let mut candidate = None;
    let mut roots = 0;
    loop {
        match reader.read_event().ok()? {
            Event::Start(element) => {
                let name = element.name().as_ref().to_owned();
                if path.is_empty() {
                    roots += 1;
                    if roots != 1 || name != "clientConfig" {
                        return None;
                    }
                }
                path.push(name);
                if path.len() > 16 {
                    return None;
                }
                if path_is(&path, &["clientConfig", "emailProvider", "incomingServer"]) {
                    let attr = element.try_get_attribute("type").ok()??;
                    server = Server {
                        imap: attr.value.as_ref() == "imap",
                        ..Default::default()
                    };
                }
            }
            Event::Text(text) => {
                let text = text.as_ref();
                append(&path, &mut server, text);
                if path.is_empty() && !text.trim().is_empty() {
                    return None;
                }
            }
            Event::CData(text) => {
                if path.is_empty() {
                    return None;
                }
                append(&path, &mut server, text.as_ref());
            }
            // INVARIANT: Resolve only XML built-ins and numeric references, never DTD entities.
            Event::GeneralRef(reference) => {
                let encoded = format!("&{};", reference.as_ref());
                let decoded = quick_xml::escape::unescape(&encoded).ok()?;
                append(&path, &mut server, &decoded);
                if path.is_empty() {
                    return None;
                }
            }
            Event::DocType(_) => return None,
            Event::End(_) => {
                if path_is(&path, &["clientConfig", "emailProvider", "incomingServer"])
                    && candidate.is_none()
                {
                    candidate = server.provider(domain);
                }
                path.pop()?;
            }
            Event::Eof => {
                return if path.is_empty() && roots == 1 {
                    candidate
                } else {
                    None
                }
            }
            _ => {}
        }
    }
}

fn path_is(path: &[String], expected: &[&str]) -> bool {
    path.iter().map(String::as_str).eq(expected.iter().copied())
}

fn append(path: &[String], server: &mut Server, value: &str) {
    if path.len() != 4
        || !path_is(
            &path[..3],
            &["clientConfig", "emailProvider", "incomingServer"],
        )
    {
        return;
    }
    match path[3].as_str() {
        "hostname" => server.hostname.push_str(value),
        "port" => server.port.push_str(value),
        "socketType" => server.socket_type.push_str(value),
        _ => {}
    }
}
