#![forbid(unsafe_code)]

//! Zero-copy RFC 5322 and MIME message parser using Stalwart Labs' `mail-parser`.
//!
//! Extracts threading headers (`Message-ID`, `In-Reply-To`, `References`),
//! RFC 8058 One-Click unsubscribe metadata, sender/recipient envelopes,
//! body texts, and lazy attachment metadata without eagerly allocating binary payloads.

use std::collections::{HashMap, HashSet};

use mail_parser::{AddressList, HeaderValue, Message, MessageParser, MessagePart};
use serde::{Deserialize, Serialize};

use crate::{
    AttachmentMeta, EngineError, MimeExtractor, ParsedInboundMessage, UnsubscribeDescriptor,
};

/// High-performance RFC 5322 / MIME parser wrapping Stalwart's `mail-parser`.
#[derive(Debug, Default, Clone, Copy)]
pub struct StalwartMimeParser;

impl StalwartMimeParser {
    /// Constructs a new `StalwartMimeParser`.
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Parses raw RFC 5322 bytes into a `ParsedInboundMessage` with specified `uid` and `modseq`.
    pub fn parse_with_uid_and_modseq(
        &self,
        raw_email: &[u8],
        uid: u32,
        modseq: Option<u64>,
    ) -> Result<ParsedInboundMessage, EngineError> {
        let message = MessageParser::default().parse(raw_email).ok_or_else(|| {
            EngineError::MimeParse("Failed to parse RFC 5322 MIME message".into())
        })?;

        // 1. Threading headers (Message-ID, In-Reply-To, References)
        let message_id_header = message.message_id().map(clean_message_id);

        let in_reply_to = message
            .in_reply_to()
            .and_then(|list| list.iter().map(clean_message_id).find(|id| !id.is_empty()));

        let references = message
            .references()
            .map(|list| {
                list.iter()
                    .map(clean_message_id)
                    .filter(|id| !id.is_empty())
                    .collect::<Vec<String>>()
            })
            .unwrap_or_default();

        let references_header = if references.is_empty() {
            None
        } else {
            Some(
                references
                    .iter()
                    .map(|id| format!("<{id}>"))
                    .collect::<Vec<_>>()
                    .join(" "),
            )
        };

        // 2. Envelopes (From, To, Cc, Bcc, Subject, Date)
        let subject = message.subject().unwrap_or_default().trim().to_string();
        let sender = format_address_list(message.from())
            .unwrap_or_else(|| format_address_list(message.sender()).unwrap_or_default());

        let mut recipients = Vec::new();
        collect_addresses(message.to(), &mut recipients);
        collect_addresses(message.cc(), &mut recipients);
        collect_addresses(message.bcc(), &mut recipients);

        let date_epoch_ms = message.date().map(|d| d.to_timestamp() * 1000);

        // 3. Plain text and HTML bodies
        let text_plain = {
            let text_bodies: Vec<String> = message
                .parts()
                .filter(|part| part.is_content_type("text", "plain"))
                .filter_map(|part| part.text())
                .map(|b| b.trim().to_string())
                .filter(|b| !b.is_empty())
                .collect();
            if text_bodies.is_empty() {
                let fallback: Vec<String> = message
                    .text_body()
                    .filter(|part| !part.is_content_type("text", "html"))
                    .filter_map(|part| part.text())
                    .map(|b| b.trim().to_string())
                    .filter(|b| !b.is_empty())
                    .collect();
                if fallback.is_empty() {
                    None
                } else {
                    Some(fallback.join("\n\n"))
                }
            } else {
                Some(text_bodies.join("\n\n"))
            }
        };

        let html_raw = {
            let html_bodies: Vec<String> = message
                .parts()
                .filter(|part| part.is_content_type("text", "html"))
                .filter_map(|part| part.html_utf8().or_else(|| part.text()))
                .map(|b| b.trim().to_string())
                .filter(|b| !b.is_empty())
                .collect();
            if html_bodies.is_empty() {
                None
            } else {
                Some(html_bodies.join("\n"))
            }
        };

        // 4. Raw headers block
        let raw_headers = match std::str::from_utf8(message.root_part().raw_headers()) {
            Ok(h) if !h.trim().is_empty() => Some(h.to_string()),
            _ => None,
        };

        // 5. RFC 2369 / RFC 8058 Unsubscribe Extraction
        let unsubscribe = extract_unsubscribe_descriptor(&message);

        // 6. Lazy Attachment Part Metadata
        let attachments = extract_lazy_attachments(&message);

        Ok(ParsedInboundMessage {
            uid,
            modseq,
            message_id_header,
            in_reply_to,
            references,
            references_header,
            subject,
            sender,
            recipients,
            date_epoch_ms,
            text_plain,
            html_raw,
            raw_headers,
            unsubscribe,
            attachments,
        })
    }
}

impl MimeExtractor for StalwartMimeParser {
    fn parse_lightweight(
        &self,
        raw_headers_and_body: &[u8],
    ) -> Result<ParsedInboundMessage, EngineError> {
        self.parse_with_uid_and_modseq(raw_headers_and_body, 0, None)
    }
}

/// Normalizes a Message-ID by trimming enclosing angle brackets and whitespace.
fn clean_message_id(raw: &str) -> String {
    raw.trim()
        .trim_matches(|c| c == '<' || c == '>')
        .to_string()
}

/// Formats the primary mailbox in an `AddressList` as `"Display Name <email@example.com>"` or `"email@example.com"`.
fn format_address_list(list: Option<AddressList<'_>>) -> Option<String> {
    let list = list?;
    let mb = list.mailboxes().next()?;
    let addr = mb.address().unwrap_or("").trim();
    if addr.is_empty() {
        return None;
    }
    match mb.name().map(str::trim).filter(|n| !n.is_empty()) {
        Some(name) => Some(format!("{name} <{addr}>")),
        None => Some(addr.to_string()),
    }
}

/// Collects formatted addresses from an `AddressList` into a target collection.
fn collect_addresses(list: Option<AddressList<'_>>, out: &mut Vec<String>) {
    if let Some(list) = list {
        for mb in list.mailboxes() {
            if let Some(addr) = mb.address().map(str::trim).filter(|a| !a.is_empty()) {
                let formatted = match mb.name().map(str::trim).filter(|n| !n.is_empty()) {
                    Some(name) => format!("{name} <{addr}>"),
                    None => addr.to_string(),
                };
                if !out.contains(&formatted) {
                    out.push(formatted);
                }
            }
        }
    }
}

/// Extracts RFC 2369 / RFC 8058 unsubscribe descriptors.
fn extract_unsubscribe_descriptor(message: &Message<'_>) -> Option<UnsubscribeDescriptor> {
    let mut raw_list_unsubscribe = None;
    let mut raw_list_unsubscribe_post = None;
    let mut https_uri = None;
    let mut mailto_uri = None;

    for header in message.headers() {
        let name_str = header.raw_name();

        if name_str.eq_ignore_ascii_case("List-Unsubscribe") {
            let val = match header.value() {
                HeaderValue::Text(t) => t.to_string(),
                _ => std::str::from_utf8(header.raw_value())
                    .unwrap_or_default()
                    .trim()
                    .to_string(),
            };
            if !val.is_empty() {
                raw_list_unsubscribe = Some(val);
            }
        } else if name_str.eq_ignore_ascii_case("List-Unsubscribe-Post") {
            let val = match header.value() {
                HeaderValue::Text(t) => t.to_string(),
                _ => std::str::from_utf8(header.raw_value())
                    .unwrap_or_default()
                    .trim()
                    .to_string(),
            };
            if !val.is_empty() {
                raw_list_unsubscribe_post = Some(val);
            }
        }
    }

    // Parse URIs from List-Unsubscribe
    if let Some(ref raw_unsub) = raw_list_unsubscribe {
        parse_unsubscribe_uris(raw_unsub, &mut https_uri, &mut mailto_uri);
    }

    // Also check structured list_unsubscribe() if URIs were not found via raw scan
    if https_uri.is_none() && mailto_uri.is_none() {
        if let Some(list) = message.list_unsubscribe() {
            for mb in list.mailboxes() {
                if let Some(addr) = mb.address() {
                    let trimmed = addr.trim();
                    if (trimmed.starts_with("https://") || trimmed.starts_with("http://"))
                        && https_uri.is_none()
                    {
                        https_uri = Some(trimmed.to_string());
                    } else if trimmed.starts_with("mailto:") && mailto_uri.is_none() {
                        mailto_uri = Some(trimmed.to_string());
                    }
                }
            }
        }
    }

    if raw_list_unsubscribe.is_none()
        && raw_list_unsubscribe_post.is_none()
        && https_uri.is_none()
        && mailto_uri.is_none()
    {
        return None;
    }

    // RFC 8058 requires BOTH List-Unsubscribe-Post: List-Unsubscribe=One-Click
    // AND a valid https:// URI in List-Unsubscribe.
    let supports_rfc8058_one_click = raw_list_unsubscribe_post
        .as_deref()
        .map(|s| s.trim().eq_ignore_ascii_case("List-Unsubscribe=One-Click"))
        .unwrap_or(false)
        && https_uri
            .as_deref()
            .map(|u| u.starts_with("https://"))
            .unwrap_or(false);

    Some(UnsubscribeDescriptor {
        https_uri,
        mailto_uri,
        supports_rfc8058_one_click,
        raw_list_unsubscribe,
        raw_list_unsubscribe_post,
    })
}

/// Parses angle-bracket delimited URIs from RFC 2369 `List-Unsubscribe` header.
fn parse_unsubscribe_uris(
    raw: &str,
    https_uri: &mut Option<String>,
    mailto_uri: &mut Option<String>,
) {
    let mut remainder = raw;
    while let Some(start) = remainder.find('<') {
        let after_start = &remainder[start + 1..];
        if let Some(end) = after_start.find('>') {
            let candidate = after_start[..end].trim();
            if (candidate.starts_with("https://") || candidate.starts_with("http://"))
                && https_uri.is_none()
            {
                *https_uri = Some(candidate.to_string());
            } else if candidate.starts_with("mailto:") && mailto_uri.is_none() {
                *mailto_uri = Some(candidate.to_string());
            }
            remainder = &after_start[end + 1..];
        } else {
            break;
        }
    }
}

/// Extracts lazy attachment part metadata from the message part hierarchy.
///
/// INVARIANT: Never reads or allocates decoded binary attachment content into memory.
/// Only part_id, filename, content_type, decoded_len, and content_id are stored.
fn extract_lazy_attachments(message: &Message<'_>) -> Vec<AttachmentMeta> {
    let mut sections = Vec::new();
    walk_part_sections(message.root_part(), "", &mut sections);

    let mut attachments = Vec::new();
    for (section, part) in sections {
        // Skip multipart structural nodes
        if part.is_multipart() {
            continue;
        }

        let is_att = part.is_attachment();
        let is_inl = part.is_inline();
        let is_non_text = !part.is_text()
            && part
                .content_type()
                .map(|c| c.ctype() != "text")
                .unwrap_or(true);

        // A part is captured if marked as attachment, inline with cid/name, or non-text binary
        if is_att || is_inl || is_non_text {
            let filename = part
                .attachment_name()
                .map(str::trim)
                .filter(|n| !n.is_empty())
                .map(str::to_string)
                .unwrap_or_else(|| format!("attachment_{section}"));

            let mime_type = part
                .content_type()
                .map(|c| match c.subtype() {
                    Some(sub) => format!("{}/{}", c.ctype(), sub),
                    None => c.ctype().to_string(),
                })
                .unwrap_or_else(|| "application/octet-stream".to_string());

            let content_id = part
                .content_id()
                .map(clean_message_id)
                .filter(|cid| !cid.is_empty());

            // INVARIANT: decoded_len() calculates the expected decoded byte size
            // without allocating or decoding the binary payload.
            let size_bytes = part.decoded_len() as u64;

            attachments.push(AttachmentMeta {
                part_id: section,
                filename,
                mime_type,
                size_bytes,
                content_id,
                is_inline: is_inl,
            });
        }
    }

    attachments
}

/// Recursively walks MIME parts to assign IMAP-compliant RFC 3501 section numbers (e.g. "1", "2", "1.1", "2.1").
fn walk_part_sections<'m>(
    part: MessagePart<'m>,
    prefix: &str,
    out: &mut Vec<(String, MessagePart<'m>)>,
) {
    if part.is_multipart() {
        for (i, child) in part.children().enumerate() {
            let child_num = i + 1;
            let section = if prefix.is_empty() {
                format!("{child_num}")
            } else {
                format!("{prefix}.{child_num}")
            };
            walk_part_sections(child, &section, out);
        }
    } else {
        let section = if prefix.is_empty() {
            "1".to_string()
        } else {
            prefix.to_string()
        };
        out.push((section, part));
    }
}

/// A node in a reconstructed conversation threading tree.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ThreadNode {
    pub message_id: String,
    pub children: Vec<ThreadNode>,
}

/// Reconstructs a hierarchical threading tree from a set of inbound parsed messages
/// based on `Message-ID`, `In-Reply-To`, and `References`.
#[must_use]
pub fn build_thread_tree(messages: &[ParsedInboundMessage]) -> Vec<ThreadNode> {
    let mut msg_map: HashMap<String, &ParsedInboundMessage> = HashMap::new();
    let mut children_map: HashMap<String, Vec<String>> = HashMap::new();
    let mut all_ids = HashSet::new();

    for msg in messages {
        if let Some(ref mid) = msg.message_id_header {
            if !mid.is_empty() {
                msg_map.insert(mid.clone(), msg);
                all_ids.insert(mid.clone());
            }
        }
    }

    // Determine parent for each message
    let mut root_ids = Vec::new();

    for msg in messages {
        let Some(ref mid) = msg.message_id_header else {
            continue;
        };
        if mid.is_empty() {
            continue;
        }

        // Try direct in_reply_to first, then fallback to last reference
        let parent_candidate = msg
            .in_reply_to
            .as_ref()
            .filter(|p| all_ids.contains(*p))
            .or_else(|| {
                msg.references
                    .iter()
                    .rev()
                    .find(|r| all_ids.contains(*r) && *r != mid)
            });

        match parent_candidate {
            Some(parent_id) => {
                children_map
                    .entry(parent_id.clone())
                    .or_default()
                    .push(mid.clone());
            }
            None => {
                root_ids.push(mid.clone());
            }
        }
    }

    // Recursively assemble ThreadNode tree
    fn assemble_node(id: &str, children_map: &HashMap<String, Vec<String>>) -> ThreadNode {
        let children = children_map
            .get(id)
            .map(|kids| {
                kids.iter()
                    .map(|kid| assemble_node(kid, children_map))
                    .collect()
            })
            .unwrap_or_default();

        ThreadNode {
            message_id: id.to_string(),
            children,
        }
    }

    root_ids
        .into_iter()
        .map(|root_id| assemble_node(&root_id, &children_map))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rfc5322_threading_and_tree_reconstruction() {
        let parser = StalwartMimeParser::new();

        let raw_root = concat!(
            "From: alice@example.com\r\n",
            "To: team@example.org\r\n",
            "Subject: Project Kickoff\r\n",
            "Message-ID: <msg-root@example.com>\r\n",
            "Date: Thu, 29 Oct 2026 10:00:00 +0000\r\n",
            "\r\n",
            "Let's start the project.\r\n"
        );

        let raw_reply1 = concat!(
            "From: bob@example.org\r\n",
            "To: alice@example.com\r\n",
            "Subject: Re: Project Kickoff\r\n",
            "Message-ID: <msg-reply-1@example.org>\r\n",
            "In-Reply-To: <msg-root@example.com>\r\n",
            "References: <msg-root@example.com>\r\n",
            "Date: Thu, 29 Oct 2026 10:30:00 +0000\r\n",
            "\r\n",
            "Sounds good, I am in.\r\n"
        );

        let raw_reply2 = concat!(
            "From: carol@example.org\r\n",
            "To: bob@example.org\r\n",
            "Subject: Re: Project Kickoff\r\n",
            "Message-ID: <msg-reply-2@example.org>\r\n",
            "In-Reply-To: <msg-reply-1@example.org>\r\n",
            "References: <msg-root@example.com> <msg-reply-1@example.org>\r\n",
            "Date: Thu, 29 Oct 2026 11:00:00 +0000\r\n",
            "\r\n",
            "Count me in as well!\r\n"
        );

        let msg_root = parser
            .parse_with_uid_and_modseq(raw_root.as_bytes(), 101, Some(1))
            .expect("parse root");
        let msg_reply1 = parser
            .parse_with_uid_and_modseq(raw_reply1.as_bytes(), 102, Some(2))
            .expect("parse reply1");
        let msg_reply2 = parser
            .parse_with_uid_and_modseq(raw_reply2.as_bytes(), 103, Some(3))
            .expect("parse reply2");

        assert_eq!(
            msg_root.message_id_header.as_deref(),
            Some("msg-root@example.com")
        );
        assert_eq!(
            msg_reply1.in_reply_to.as_deref(),
            Some("msg-root@example.com")
        );
        assert_eq!(
            msg_reply2.references,
            vec!["msg-root@example.com", "msg-reply-1@example.org"]
        );

        let tree = build_thread_tree(&[msg_root, msg_reply1, msg_reply2]);
        assert_eq!(tree.len(), 1);
        assert_eq!(tree[0].message_id, "msg-root@example.com");
        assert_eq!(tree[0].children.len(), 1);
        assert_eq!(tree[0].children[0].message_id, "msg-reply-1@example.org");
        assert_eq!(tree[0].children[0].children.len(), 1);
        assert_eq!(
            tree[0].children[0].children[0].message_id,
            "msg-reply-2@example.org"
        );
    }

    #[test]
    fn test_rfc8058_one_click_unsubscribe_extraction() {
        let parser = StalwartMimeParser::new();

        let raw_newsletter = concat!(
            "From: newsletter@example.org\r\n",
            "To: subscriber@example.com\r\n",
            "Subject: Weekly Digest\r\n",
            "Message-ID: <digest-42@example.org>\r\n",
            "List-Unsubscribe: <https://example.org/unsub?token=xyz123>, <mailto:unsub@example.org?subject=unsub>\r\n",
            "List-Unsubscribe-Post: List-Unsubscribe=One-Click\r\n",
            "\r\n",
            "Here is your weekly digest content.\r\n"
        );

        let parsed = parser
            .parse_lightweight(raw_newsletter.as_bytes())
            .expect("parse");
        let unsub = parsed.unsubscribe.expect("unsubscribe descriptor");

        assert!(unsub.supports_rfc8058_one_click);
        assert_eq!(
            unsub.https_uri.as_deref(),
            Some("https://example.org/unsub?token=xyz123")
        );
        assert_eq!(
            unsub.mailto_uri.as_deref(),
            Some("mailto:unsub@example.org?subject=unsub")
        );
        assert_eq!(
            unsub.raw_list_unsubscribe_post.as_deref(),
            Some("List-Unsubscribe=One-Click")
        );
    }

    #[test]
    fn test_rfc2369_mailto_only_unsubscribe_not_one_click() {
        let parser = StalwartMimeParser::new();

        let raw_email = concat!(
            "From: list@example.org\r\n",
            "To: subscriber@example.com\r\n",
            "Subject: Old School Mailing List\r\n",
            "Message-ID: <legacy-1@example.org>\r\n",
            "List-Unsubscribe: <mailto:leave@example.org?subject=unsubscribe>\r\n",
            "\r\n",
            "Legacy body.\r\n"
        );

        let parsed = parser
            .parse_lightweight(raw_email.as_bytes())
            .expect("parse");
        let unsub = parsed.unsubscribe.expect("unsubscribe descriptor");

        assert!(!unsub.supports_rfc8058_one_click);
        assert_eq!(unsub.https_uri, None);
        assert_eq!(
            unsub.mailto_uri.as_deref(),
            Some("mailto:leave@example.org?subject=unsubscribe")
        );
    }

    #[test]
    fn test_lazy_attachment_metadata_without_payload_download() {
        let parser = StalwartMimeParser::new();

        let raw_multipart = concat!(
            "From: billing@example.org\r\n",
            "To: finance@example.com\r\n",
            "Subject: Invoice Attached\r\n",
            "Message-ID: <invoice-101@example.org>\r\n",
            "MIME-Version: 1.0\r\n",
            "Content-Type: multipart/mixed; boundary=\"BOUNDARY_XYZ\"\r\n",
            "\r\n",
            "--BOUNDARY_XYZ\r\n",
            "Content-Type: text/plain; charset=utf-8\r\n",
            "\r\n",
            "Please find attached your invoice.\r\n",
            "--BOUNDARY_XYZ\r\n",
            "Content-Type: application/pdf; name=\"fatura_2026_10.pdf\"\r\n",
            "Content-Disposition: attachment; filename=\"fatura_2026_10.pdf\"\r\n",
            "Content-Transfer-Encoding: base64\r\n",
            "\r\n",
            "JVBERi0xLjQKJcTl8uXr...\r\n",
            "--BOUNDARY_XYZ\r\n",
            "Content-Type: image/png\r\n",
            "Content-Disposition: inline; filename=\"company_logo.png\"\r\n",
            "Content-ID: <logo_cid_99@example.org>\r\n",
            "Content-Transfer-Encoding: base64\r\n",
            "\r\n",
            "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==\r\n",
            "--BOUNDARY_XYZ--\r\n"
        );

        let parsed = parser
            .parse_lightweight(raw_multipart.as_bytes())
            .expect("parse");
        assert_eq!(parsed.attachments.len(), 2);

        // First attachment: PDF
        let pdf = &parsed.attachments[0];
        assert_eq!(pdf.part_id, "2");
        assert_eq!(pdf.filename, "fatura_2026_10.pdf");
        assert_eq!(pdf.mime_type, "application/pdf");
        assert!(!pdf.is_inline);
        assert_eq!(pdf.content_id, None);
        assert!(pdf.size_bytes > 0);

        // Second attachment: Inline PNG with CID
        let logo = &parsed.attachments[1];
        assert_eq!(logo.part_id, "3");
        assert_eq!(logo.filename, "company_logo.png");
        assert_eq!(logo.mime_type, "image/png");
        assert!(logo.is_inline);
        assert_eq!(logo.content_id.as_deref(), Some("logo_cid_99@example.org"));
        assert!(logo.size_bytes > 0);
    }
}
