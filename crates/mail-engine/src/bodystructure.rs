// INVARIANT: IMAP RFC 3501 BODYSTRUCTURE responses must be parsed safely into structural metadata
// without allocating or decoding binary attachment payloads.
// INVARIANT: Attachment metadata (part_id, filename, content_type, size_bytes, content_id) is extracted
// strictly from BODYSTRUCTURE to enable zero-copy Lazy Attachments during inbound synchronization.

use std::collections::HashMap;

use mail_parser::MessageParser;

use crate::mime_parser::{clean_message_id, format_address_list};
use crate::{AttachmentMeta, EngineError, MimeExtractor, ParsedInboundMessage};

/// Represents an abstract S-expression token in IMAP RFC 3501 protocol responses.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SExpr {
    Atom(String),
    String(String),
    Number(u64),
    Nil,
    List(Vec<SExpr>),
}

/// Tokenizer token for S-expressions.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Token {
    LParen,
    RParen,
    Atom(String),
    String(String),
    Number(u64),
    Nil,
}

/// Tokenizes an S-expression string into tokens.
fn tokenize_sexpr(input: &str) -> Result<Vec<Token>, EngineError> {
    let mut tokens = Vec::new();
    let bytes = input.as_bytes();
    let mut i = 0;

    while i < bytes.len() {
        match bytes[i] {
            b' ' | b'\t' | b'\r' | b'\n' => {
                i += 1;
            }
            b'(' => {
                tokens.push(Token::LParen);
                i += 1;
            }
            b')' => {
                tokens.push(Token::RParen);
                i += 1;
            }
            b'"' => {
                i += 1;
                let mut string_buf = String::new();
                let mut escaped = false;
                while i < bytes.len() {
                    let b = bytes[i];
                    if escaped {
                        string_buf.push(b as char);
                        escaped = false;
                        i += 1;
                    } else if b == b'\\' {
                        escaped = true;
                        i += 1;
                    } else if b == b'"' {
                        i += 1;
                        break;
                    } else {
                        string_buf.push(b as char);
                        i += 1;
                    }
                }
                tokens.push(Token::String(string_buf));
            }
            b'{' => {
                // IMAP literal format: {<size>}\r\n<bytes>
                let start_size = i + 1;
                while i < bytes.len() && bytes[i] != b'}' {
                    i += 1;
                }
                if i >= bytes.len() {
                    return Err(EngineError::ImapProtocol(
                        "malformed literal in S-expression: unclosed '}'".into(),
                    ));
                }
                let size_str = &input[start_size..i];
                let size: usize = size_str.parse().map_err(|_| {
                    EngineError::ImapProtocol(format!("invalid literal size `{size_str}`"))
                })?;
                i += 1; // skip '}'
                if i + 1 < bytes.len() && bytes[i] == b'\r' && bytes[i + 1] == b'\n' {
                    i += 2;
                } else if i < bytes.len() && bytes[i] == b'\n' {
                    i += 1;
                }
                let end = (i + size).min(bytes.len());
                let literal_str = String::from_utf8_lossy(&bytes[i..end]).to_string();
                tokens.push(Token::String(literal_str));
                i = end;
            }
            _ => {
                let start = i;
                while i < bytes.len()
                    && !matches!(
                        bytes[i],
                        b' ' | b'\t' | b'\r' | b'\n' | b'(' | b')' | b'"' | b'{'
                    )
                {
                    i += 1;
                }
                let token_str = &input[start..i];
                if token_str.eq_ignore_ascii_case("NIL") {
                    tokens.push(Token::Nil);
                } else if let Ok(n) = token_str.parse::<u64>() {
                    tokens.push(Token::Number(n));
                } else {
                    tokens.push(Token::Atom(token_str.to_string()));
                }
            }
        }
    }

    Ok(tokens)
}

/// Parses a token stream into an `SExpr` tree.
fn parse_sexpr_from_tokens(tokens: &[Token], pos: &mut usize) -> Result<SExpr, EngineError> {
    if *pos >= tokens.len() {
        return Err(EngineError::ImapProtocol(
            "unexpected end of S-expression".into(),
        ));
    }
    match &tokens[*pos] {
        Token::LParen => {
            *pos += 1;
            let mut list = Vec::new();
            while *pos < tokens.len() && tokens[*pos] != Token::RParen {
                list.push(parse_sexpr_from_tokens(tokens, pos)?);
            }
            if *pos >= tokens.len() {
                return Err(EngineError::ImapProtocol(
                    "unclosed parenthesis in S-expression".into(),
                ));
            }
            *pos += 1; // consume RParen
            Ok(SExpr::List(list))
        }
        Token::RParen => Err(EngineError::ImapProtocol(
            "unexpected ')' in S-expression".into(),
        )),
        Token::Atom(s) => {
            *pos += 1;
            Ok(SExpr::Atom(s.clone()))
        }
        Token::String(s) => {
            *pos += 1;
            Ok(SExpr::String(s.clone()))
        }
        Token::Number(n) => {
            *pos += 1;
            Ok(SExpr::Number(*n))
        }
        Token::Nil => {
            *pos += 1;
            Ok(SExpr::Nil)
        }
    }
}

/// Content disposition extracted from RFC 3501 `body-fld-dsp`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Disposition {
    pub disposition_type: String,
    pub parameters: Vec<(String, String)>,
}

/// Single-part body structure (`body-type-1part`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SinglePartStructure {
    pub media_type: String,
    pub media_subtype: String,
    pub parameters: Vec<(String, String)>,
    pub id: Option<String>,
    pub description: Option<String>,
    pub encoding: String,
    pub octets: u64,
    pub lines: Option<u64>,
    pub disposition: Option<Disposition>,
}

/// Multipart body structure (`body-type-mpart`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MultipartStructure {
    pub parts: Vec<BodyPartStructure>,
    pub media_subtype: String,
    pub parameters: Vec<(String, String)>,
    pub disposition: Option<Disposition>,
}

/// Hierarchical body part structure (RFC 3501 Section 7.4.2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BodyPartStructure {
    Single(SinglePartStructure),
    Multipart(MultipartStructure),
}

/// Extracts key-value parameter pairs from S-expression lists.
fn parse_param_pairs(expr: &SExpr) -> Vec<(String, String)> {
    let mut pairs = Vec::new();
    if let SExpr::List(items) = expr {
        let mut i = 0;
        while i + 1 < items.len() {
            let key = match &items[i] {
                SExpr::Atom(s) | SExpr::String(s) => s.clone(),
                _ => break,
            };
            let val = match &items[i + 1] {
                SExpr::Atom(s) | SExpr::String(s) => s.clone(),
                SExpr::Number(n) => n.to_string(),
                _ => String::new(),
            };
            pairs.push((key, val));
            i += 2;
        }
    }
    pairs
}

/// Parses a disposition S-expression (e.g. `("ATTACHMENT" ("FILENAME" "doc.pdf"))`).
fn parse_disposition(expr: &SExpr) -> Option<Disposition> {
    if let SExpr::List(items) = expr {
        if !items.is_empty() {
            let disp_type = match &items[0] {
                SExpr::Atom(s) | SExpr::String(s) => s.to_ascii_uppercase(),
                _ => return None,
            };
            let params = if items.len() > 1 {
                parse_param_pairs(&items[1])
            } else {
                Vec::new()
            };
            return Some(Disposition {
                disposition_type: disp_type,
                parameters: params,
            });
        }
    }
    None
}

/// Parses an `SExpr` representing a `BODYSTRUCTURE` into `BodyPartStructure`.
pub fn parse_body_part_structure(expr: &SExpr) -> Result<BodyPartStructure, EngineError> {
    match expr {
        SExpr::List(items) => {
            if items.is_empty() {
                return Err(EngineError::ImapProtocol("empty BODYSTRUCTURE list".into()));
            }

            // RFC 3501: If first element is a list, it is a multipart.
            if matches!(&items[0], SExpr::List(_)) {
                let mut child_parts = Vec::new();
                let mut i = 0;
                while i < items.len() && matches!(&items[i], SExpr::List(_)) {
                    child_parts.push(parse_body_part_structure(&items[i])?);
                    i += 1;
                }
                if i >= items.len() {
                    return Err(EngineError::ImapProtocol(
                        "malformed multipart BODYSTRUCTURE: missing subtype".into(),
                    ));
                }
                let media_subtype = match &items[i] {
                    SExpr::Atom(s) | SExpr::String(s) => s.clone(),
                    _ => {
                        return Err(EngineError::ImapProtocol(
                            "malformed multipart BODYSTRUCTURE: expected subtype string".into(),
                        ))
                    }
                };
                i += 1;

                let parameters = if i < items.len() {
                    parse_param_pairs(&items[i])
                } else {
                    Vec::new()
                };
                i += 1;

                let disposition = if i < items.len() {
                    parse_disposition(&items[i])
                } else {
                    None
                };

                Ok(BodyPartStructure::Multipart(MultipartStructure {
                    parts: child_parts,
                    media_subtype,
                    parameters,
                    disposition,
                }))
            } else {
                // Single-part: media_type, media_subtype, parameters, id, desc, encoding, octets...
                if items.len() < 7 {
                    return Err(EngineError::ImapProtocol(format!(
                        "malformed single-part BODYSTRUCTURE: expected >= 7 fields, got {}",
                        items.len()
                    )));
                }

                let media_type = match &items[0] {
                    SExpr::Atom(s) | SExpr::String(s) => s.clone(),
                    _ => {
                        return Err(EngineError::ImapProtocol(
                            "expected media type in single-part BODYSTRUCTURE".into(),
                        ))
                    }
                };
                let media_subtype = match &items[1] {
                    SExpr::Atom(s) | SExpr::String(s) => s.clone(),
                    _ => {
                        return Err(EngineError::ImapProtocol(
                            "expected media subtype in single-part BODYSTRUCTURE".into(),
                        ))
                    }
                };
                let parameters = parse_param_pairs(&items[2]);
                let id = match &items[3] {
                    SExpr::String(s) | SExpr::Atom(s) if !s.is_empty() => Some(s.clone()),
                    _ => None,
                };
                let description = match &items[4] {
                    SExpr::String(s) | SExpr::Atom(s) if !s.is_empty() => Some(s.clone()),
                    _ => None,
                };
                let encoding = match &items[5] {
                    SExpr::Atom(s) | SExpr::String(s) => s.clone(),
                    _ => "7BIT".to_string(),
                };
                let octets = match &items[6] {
                    SExpr::Number(n) => *n,
                    SExpr::Atom(s) => s.parse().unwrap_or(0),
                    _ => 0,
                };

                let mut lines = None;
                let mut disposition = None;

                if media_type.eq_ignore_ascii_case("TEXT") {
                    if items.len() > 7 {
                        lines = match &items[7] {
                            SExpr::Number(n) => Some(*n),
                            SExpr::Atom(s) => s.parse().ok(),
                            _ => None,
                        };
                    }
                    if items.len() > 9 {
                        disposition = parse_disposition(&items[9]);
                    }
                } else if items.len() > 8 {
                    disposition = parse_disposition(&items[8]);
                }

                // Fallback scan for disposition list if not located at standard position
                if disposition.is_none() && items.len() > 7 {
                    for item in &items[7..] {
                        if let Some(dsp) = parse_disposition(item) {
                            disposition = Some(dsp);
                            break;
                        }
                    }
                }

                Ok(BodyPartStructure::Single(SinglePartStructure {
                    media_type,
                    media_subtype,
                    parameters,
                    id,
                    description,
                    encoding,
                    octets,
                    lines,
                    disposition,
                }))
            }
        }
        _ => Err(EngineError::ImapProtocol(
            "BODYSTRUCTURE must be a parenthesized list".into(),
        )),
    }
}

/// Parses a raw BODYSTRUCTURE S-expression string.
pub fn parse_bodystructure_str(input: &str) -> Result<BodyPartStructure, EngineError> {
    let tokens = tokenize_sexpr(input)?;
    let mut pos = 0;
    let sexpr = parse_sexpr_from_tokens(&tokens, &mut pos)?;
    parse_body_part_structure(&sexpr)
}

/// Finds and extracts the substring containing the complete `BODYSTRUCTURE (...)` S-expression.
pub fn extract_bodystructure_sexpr(input: &str) -> Option<&str> {
    let upper = input.to_ascii_uppercase();
    let marker = "BODYSTRUCTURE ";
    let pos = upper.find(marker)?;
    let start_idx = pos + marker.len();
    let remaining = &input[start_idx..];

    let mut start_paren = None;
    for (idx, ch) in remaining.char_indices() {
        if ch == '(' {
            start_paren = Some(start_idx + idx);
            break;
        } else if !ch.is_whitespace() {
            return None;
        }
    }

    let start = start_paren?;
    let bytes = input.as_bytes();
    let mut depth = 0;
    let mut in_quote = false;
    let mut escaped = false;
    let mut end = None;

    for (i, &b) in bytes.iter().enumerate().skip(start) {
        if in_quote {
            if escaped {
                escaped = false;
            } else if b == b'\\' {
                escaped = true;
            } else if b == b'"' {
                in_quote = false;
            }
        } else {
            match b {
                b'"' => in_quote = true,
                b'(' => depth += 1,
                b')' => {
                    depth -= 1;
                    if depth == 0 {
                        end = Some(i);
                        break;
                    }
                }
                _ => {}
            }
        }
    }

    end.map(|e| &input[start..=e])
}

/// Information about a text section that should be selectively fetched.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextSectionInfo {
    pub section: String,
    pub media_type: String,
    pub octets: u64,
}

/// Extracted metadata from a `BODYSTRUCTURE` tree.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ExtractedBodyStructure {
    pub attachments: Vec<AttachmentMeta>,
    pub text_sections: Vec<TextSectionInfo>,
    pub is_single_part: bool,
}

/// Maximum size in bytes (2 MiB) of a text section fetched during bulk sync.
pub const MAX_EAGER_TEXT_SIZE_BYTES: u64 = 2 * 1024 * 1024;

/// Walks a `BodyPartStructure` to extract lazy attachment descriptors and candidate body text sections.
pub fn extract_body_structure_metadata(structure: &BodyPartStructure) -> ExtractedBodyStructure {
    let mut extracted = ExtractedBodyStructure::default();
    match structure {
        BodyPartStructure::Single(single) => {
            extracted.is_single_part = true;
            classify_part(single, "1", &mut extracted);
        }
        BodyPartStructure::Multipart(multi) => {
            extracted.is_single_part = false;
            walk_multipart_parts(multi, "", &mut extracted);
        }
    }
    extracted
}

fn walk_multipart_parts(
    multi: &MultipartStructure,
    prefix: &str,
    out: &mut ExtractedBodyStructure,
) {
    for (i, child) in multi.parts.iter().enumerate() {
        let child_num = i + 1;
        let section = if prefix.is_empty() {
            format!("{child_num}")
        } else {
            format!("{prefix}.{child_num}")
        };
        match child {
            BodyPartStructure::Single(single) => {
                classify_part(single, &section, out);
            }
            BodyPartStructure::Multipart(nested) => {
                walk_multipart_parts(nested, &section, out);
            }
        }
    }
}

fn classify_part(single: &SinglePartStructure, section: &str, out: &mut ExtractedBodyStructure) {
    let is_dsp_att = single
        .disposition
        .as_ref()
        .is_some_and(|d| d.disposition_type.eq_ignore_ascii_case("ATTACHMENT"));
    let is_dsp_inl = single
        .disposition
        .as_ref()
        .is_some_and(|d| d.disposition_type.eq_ignore_ascii_case("INLINE"));
    let has_filename = single
        .disposition
        .as_ref()
        .and_then(|d| {
            d.parameters
                .iter()
                .find(|(k, _)| k.eq_ignore_ascii_case("FILENAME"))
        })
        .is_some()
        || single
            .parameters
            .iter()
            .any(|(k, _)| k.eq_ignore_ascii_case("NAME") || k.eq_ignore_ascii_case("FILENAME"));
    let is_non_text = !single.media_type.eq_ignore_ascii_case("TEXT");

    let is_attachment = is_dsp_att
        || is_non_text
        || (is_dsp_inl && (has_filename || single.id.is_some()))
        || (single.media_type.eq_ignore_ascii_case("TEXT") && has_filename);

    if is_attachment {
        let mut filename = None;
        if let Some(dsp) = &single.disposition {
            for (k, v) in &dsp.parameters {
                if k.eq_ignore_ascii_case("FILENAME") {
                    filename = Some(v.clone());
                    break;
                }
            }
        }
        if filename.is_none() {
            for (k, v) in &single.parameters {
                if k.eq_ignore_ascii_case("NAME") || k.eq_ignore_ascii_case("FILENAME") {
                    filename = Some(v.clone());
                    break;
                }
            }
        }
        let filename = filename
            .filter(|n| !n.trim().is_empty())
            .unwrap_or_else(|| format!("attachment_{section}"));

        let mime_type = format!(
            "{}/{}",
            single.media_type.to_ascii_lowercase(),
            single.media_subtype.to_ascii_lowercase()
        );

        let size_bytes = if single.encoding.eq_ignore_ascii_case("BASE64") {
            single.octets.saturating_mul(3) / 4
        } else {
            single.octets
        };

        let content_id = single
            .id
            .as_deref()
            .map(clean_message_id)
            .filter(|cid| !cid.is_empty());

        out.attachments.push(AttachmentMeta {
            part_id: section.to_string(),
            filename,
            mime_type,
            size_bytes,
            content_id,
            is_inline: is_dsp_inl,
        });
    } else if single.media_type.eq_ignore_ascii_case("TEXT") {
        let subtype = single.media_subtype.to_ascii_lowercase();
        if (subtype == "plain" || subtype == "html") && single.octets <= MAX_EAGER_TEXT_SIZE_BYTES {
            out.text_sections.push(TextSectionInfo {
                section: section.to_string(),
                media_type: format!("text/{subtype}"),
                octets: single.octets,
            });
        }
    }
}

/// Assembles a `ParsedInboundMessage` from header bytes, BODYSTRUCTURE metadata, and fetched section bodies.
pub fn assemble_parsed_inbound_message(
    uid: u32,
    modseq: Option<u64>,
    header_bytes: &[u8],
    extracted_bs: Option<&ExtractedBodyStructure>,
    sections: &HashMap<String, Vec<u8>>,
    fallback_body: Option<&[u8]>,
) -> Result<ParsedInboundMessage, EngineError> {
    let header_msg = MessageParser::default()
        .parse(header_bytes)
        .ok_or_else(|| EngineError::MimeParse("failed parsing RFC 5322 header block".into()))?;

    let message_id_header = header_msg.message_id().map(clean_message_id);
    let in_reply_to = header_msg
        .in_reply_to()
        .and_then(|list| list.iter().map(clean_message_id).find(|id| !id.is_empty()));
    let references = header_msg
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

    let subject = header_msg.subject().unwrap_or_default().trim().to_string();
    let sender = format_address_list(header_msg.from())
        .unwrap_or_else(|| format_address_list(header_msg.sender()).unwrap_or_default());

    let mut recipients = Vec::new();
    if let Some(list) = header_msg.to() {
        for mb in list.mailboxes() {
            if let Some(addr) = mb.address().map(str::trim).filter(|a| !a.is_empty()) {
                let formatted = match mb.name().map(str::trim).filter(|n| !n.is_empty()) {
                    Some(name) => format!("{name} <{addr}>"),
                    None => addr.to_string(),
                };
                if !recipients.contains(&formatted) {
                    recipients.push(formatted);
                }
            }
        }
    }
    if let Some(list) = header_msg.cc() {
        for mb in list.mailboxes() {
            if let Some(addr) = mb.address().map(str::trim).filter(|a| !a.is_empty()) {
                let formatted = match mb.name().map(str::trim).filter(|n| !n.is_empty()) {
                    Some(name) => format!("{name} <{addr}>"),
                    None => addr.to_string(),
                };
                if !recipients.contains(&formatted) {
                    recipients.push(formatted);
                }
            }
        }
    }

    let date_epoch_ms = header_msg.date().map(|d| d.to_timestamp() * 1000);
    let raw_headers = match std::str::from_utf8(header_bytes) {
        Ok(h) if !h.trim().is_empty() => Some(h.to_string()),
        _ => None,
    };
    let unsubscribe = crate::mime_parser::extract_unsubscribe_descriptor(&header_msg);

    let mut text_plain = None;
    let mut html_raw = None;
    let mut attachments = Vec::new();

    if let Some(bs) = extracted_bs {
        attachments = bs.attachments.clone();

        if bs.is_single_part {
            let body_data = sections
                .get("1")
                .map(|v| v.as_slice())
                .or(fallback_body)
                .unwrap_or_default();

            let mut combined = Vec::with_capacity(header_bytes.len() + body_data.len() + 4);
            combined.extend_from_slice(header_bytes);
            if !header_bytes.ends_with(b"\r\n\r\n") {
                if header_bytes.ends_with(b"\r\n") {
                    combined.extend_from_slice(b"\r\n");
                } else {
                    combined.extend_from_slice(b"\r\n\r\n");
                }
            }
            combined.extend_from_slice(body_data);

            if let Some(parsed) = MessageParser::default().parse(&combined) {
                text_plain = parsed
                    .parts()
                    .filter(|p| p.is_content_type("text", "plain"))
                    .filter_map(|p| p.text())
                    .map(|s| s.trim().to_string())
                    .find(|s| !s.is_empty())
                    .or_else(|| {
                        parsed
                            .text_body()
                            .filter(|p| !p.is_content_type("text", "html"))
                            .filter_map(|p| p.text())
                            .map(|s| s.trim().to_string())
                            .find(|s| !s.is_empty())
                    });
                html_raw = parsed
                    .parts()
                    .filter(|p| p.is_content_type("text", "html"))
                    .filter_map(|p| p.html_utf8().or_else(|| p.text()))
                    .map(|s| s.trim().to_string())
                    .find(|s| !s.is_empty());
            }
        } else {
            let mut plain_parts = Vec::new();
            let mut html_parts = Vec::new();

            for sec in &bs.text_sections {
                let mime_bytes = sections
                    .get(&format!("{}.MIME", sec.section))
                    .map(|v| v.as_slice())
                    .unwrap_or_default();
                let body_bytes = sections
                    .get(&sec.section)
                    .map(|v| v.as_slice())
                    .unwrap_or_default();

                let mut entity = Vec::with_capacity(mime_bytes.len() + body_bytes.len() + 64);
                if !mime_bytes.is_empty() {
                    entity.extend_from_slice(mime_bytes);
                    if !mime_bytes.ends_with(b"\r\n\r\n") {
                        if mime_bytes.ends_with(b"\r\n") {
                            entity.extend_from_slice(b"\r\n");
                        } else {
                            entity.extend_from_slice(b"\r\n\r\n");
                        }
                    }
                } else {
                    let synth_header =
                        format!("Content-Type: {}; charset=utf-8\r\n\r\n", sec.media_type);
                    entity.extend_from_slice(synth_header.as_bytes());
                }
                entity.extend_from_slice(body_bytes);

                if let Some(part_msg) = MessageParser::default().parse(&entity) {
                    if sec.media_type == "text/plain" {
                        if let Some(txt) = part_msg.parts().filter_map(|p| p.text()).next() {
                            let trimmed = txt.trim().to_string();
                            if !trimmed.is_empty() {
                                plain_parts.push(trimmed);
                            }
                        } else if let Ok(s) = std::str::from_utf8(body_bytes) {
                            let trimmed = s.trim().to_string();
                            if !trimmed.is_empty() {
                                plain_parts.push(trimmed);
                            }
                        }
                    } else if sec.media_type == "text/html" {
                        if let Some(html) = part_msg
                            .parts()
                            .filter_map(|p| p.html_utf8().or_else(|| p.text()))
                            .next()
                        {
                            let trimmed = html.trim().to_string();
                            if !trimmed.is_empty() {
                                html_parts.push(trimmed);
                            }
                        } else if let Ok(s) = std::str::from_utf8(body_bytes) {
                            let trimmed = s.trim().to_string();
                            if !trimmed.is_empty() {
                                html_parts.push(trimmed);
                            }
                        }
                    }
                }
            }

            if !plain_parts.is_empty() {
                text_plain = Some(plain_parts.join("\n\n"));
            }
            if !html_parts.is_empty() {
                html_raw = Some(html_parts.join("\n"));
            }
        }
    } else {
        // Fallback: parse combined header and body payload
        let mut combined = Vec::new();
        combined.extend_from_slice(header_bytes);
        if let Some(b) = fallback_body {
            if !header_bytes.ends_with(b"\r\n\r\n") {
                combined.extend_from_slice(b"\r\n\r\n");
            }
            combined.extend_from_slice(b);
        }
        if let Ok(parsed) = crate::mime_parser::StalwartMimeParser.parse_lightweight(&combined) {
            text_plain = parsed.text_plain;
            html_raw = parsed.html_raw;
            attachments = parsed.attachments;
        }
    }

    Ok(ParsedInboundMessage {
        uid,
        modseq,
        provider_message_id: None,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_single_part_text_plain() {
        let input = r#"("TEXT" "PLAIN" ("CHARSET" "UTF-8") NIL NIL "7BIT" 120 4 NIL NIL NIL NIL)"#;
        let structure = parse_bodystructure_str(input).unwrap();
        let metadata = extract_body_structure_metadata(&structure);

        assert!(metadata.is_single_part);
        assert_eq!(metadata.attachments.len(), 0);
        assert_eq!(metadata.text_sections.len(), 1);
        assert_eq!(metadata.text_sections[0].section, "1");
        assert_eq!(metadata.text_sections[0].media_type, "text/plain");
        assert_eq!(metadata.text_sections[0].octets, 120);
    }

    #[test]
    fn parses_multipart_mixed_with_pdf_attachment() {
        let input = r#"((("TEXT" "PLAIN" ("CHARSET" "UTF-8") NIL NIL "7BIT" 50 2) ("TEXT" "HTML" ("CHARSET" "UTF-8") NIL NIL "QUOTED-PRINTABLE" 150 5) "ALTERNATIVE" ("BOUNDARY" "b1") NIL NIL) ("APPLICATION" "PDF" ("NAME" "invoice.pdf") "<cid-pdf>" NIL "BASE64" 4000 NIL ("ATTACHMENT" ("FILENAME" "invoice.pdf")) NIL) "MIXED" ("BOUNDARY" "b2") NIL NIL)"#;
        let structure = parse_bodystructure_str(input).unwrap();
        let metadata = extract_body_structure_metadata(&structure);

        assert!(!metadata.is_single_part);
        assert_eq!(metadata.text_sections.len(), 2);
        assert_eq!(metadata.text_sections[0].section, "1.1");
        assert_eq!(metadata.text_sections[0].media_type, "text/plain");
        assert_eq!(metadata.text_sections[1].section, "1.2");
        assert_eq!(metadata.text_sections[1].media_type, "text/html");

        assert_eq!(metadata.attachments.len(), 1);
        let att = &metadata.attachments[0];
        assert_eq!(att.part_id, "2");
        assert_eq!(att.filename, "invoice.pdf");
        assert_eq!(att.mime_type, "application/pdf");
        assert_eq!(att.size_bytes, 3000); // 4000 * 3 / 4
        assert_eq!(att.content_id.as_deref(), Some("cid-pdf"));
        assert!(!att.is_inline);
    }

    #[test]
    fn parses_inline_image_attachment() {
        let input = r#"(("TEXT" "HTML" ("CHARSET" "UTF-8") NIL NIL "7BIT" 200 6) ("IMAGE" "PNG" ("NAME" "logo.png") "<logo-cid>" NIL "BASE64" 8000 NIL ("INLINE" ("FILENAME" "logo.png")) NIL) "RELATED" ("BOUNDARY" "rel1") NIL NIL)"#;
        let structure = parse_bodystructure_str(input).unwrap();
        let metadata = extract_body_structure_metadata(&structure);

        assert_eq!(metadata.text_sections.len(), 1);
        assert_eq!(metadata.text_sections[0].section, "1");
        assert_eq!(metadata.attachments.len(), 1);

        let att = &metadata.attachments[0];
        assert_eq!(att.part_id, "2");
        assert_eq!(att.filename, "logo.png");
        assert_eq!(att.mime_type, "image/png");
        assert_eq!(att.size_bytes, 6000); // 8000 * 3 / 4
        assert_eq!(att.content_id.as_deref(), Some("logo-cid"));
        assert!(att.is_inline);
    }

    #[test]
    fn extracts_bodystructure_sexpr_from_fetch_line() {
        let line = r#"* 1 FETCH (UID 105 MODSEQ (888) FLAGS (\Seen) BODYSTRUCTURE ("TEXT" "PLAIN" ("CHARSET" "UTF-8") NIL NIL "7BIT" 120 4 NIL NIL NIL NIL) BODY[HEADER] {50}"#;
        let extracted = extract_bodystructure_sexpr(line).unwrap();
        assert_eq!(
            extracted,
            r#"("TEXT" "PLAIN" ("CHARSET" "UTF-8") NIL NIL "7BIT" 120 4 NIL NIL NIL NIL)"#
        );
    }

    #[test]
    fn assembles_parsed_message_with_selective_sections() {
        let headers = concat!(
            "From: sender@example.com\r\n",
            "To: recipient@example.com\r\n",
            "Subject: Test Subject\r\n",
            "Message-ID: <msg-101@example.com>\r\n\r\n"
        );
        let bs_str = r#"(("TEXT" "PLAIN" ("CHARSET" "UTF-8") NIL NIL "7BIT" 20 1) ("APPLICATION" "PDF" ("NAME" "report.pdf") NIL NIL "BASE64" 4000 NIL ("ATTACHMENT" ("FILENAME" "report.pdf")) NIL) "MIXED" ("BOUNDARY" "b") NIL NIL)"#;
        let structure = parse_bodystructure_str(bs_str).unwrap();
        let metadata = extract_body_structure_metadata(&structure);

        let mut sections = HashMap::new();
        sections.insert(
            "1.MIME".to_string(),
            b"Content-Type: text/plain; charset=utf-8\r\n\r\n".to_vec(),
        );
        sections.insert("1".to_string(), b"Hello world text body".to_vec());

        let msg = assemble_parsed_inbound_message(
            101,
            Some(500),
            headers.as_bytes(),
            Some(&metadata),
            &sections,
            None,
        )
        .unwrap();

        assert_eq!(msg.uid, 101);
        assert_eq!(msg.modseq, Some(500));
        assert_eq!(msg.subject, "Test Subject");
        assert_eq!(msg.sender, "sender@example.com");
        assert_eq!(msg.text_plain.as_deref(), Some("Hello world text body"));
        assert_eq!(msg.attachments.len(), 1);
        assert_eq!(msg.attachments[0].filename, "report.pdf");
        assert_eq!(msg.attachments[0].part_id, "2");
        assert_eq!(msg.attachments[0].size_bytes, 3000);
    }
}
