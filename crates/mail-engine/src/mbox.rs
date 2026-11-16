#![forbid(unsafe_code)]

//! Bounded streaming `.mbox` and `.eml` local archive parser.
//!
//! Splits `.mbox` streams on `From ` envelope lines without loading the entire
//! archive into memory, normalizes CRLF/LF line endings, unescapes `>From ` body
//! lines (`mboxo` / `mboxrd`), enforces a configurable per-message size cap,
//! extracts `X-Gmail-Labels`, and delegates RFC 5322 / MIME parsing to
//! [`StalwartMimeParser`].

use std::io::{BufRead, Read};

use mail_parser::{HeaderValue, MessageParser};
use serde::{Deserialize, Serialize};

use crate::{EngineError, ParsedInboundMessage, StalwartMimeParser};

/// Default per-message byte cap (32 MiB) when streaming `.mbox` or `.eml` archives.
pub const DEFAULT_MAX_MBOX_MESSAGE_BYTES: usize = 32 * 1024 * 1024;

/// Supported local email archive file formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LocalArchiveFormat {
    Mbox,
    Eml,
}

/// Strategy for unescaping `>From ` lines in `.mbox` message bodies.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum MboxFlavor {
    /// Strip a single leading `>` only from lines starting with exact `>From `.
    Mboxo,
    /// Strip one leading `>` from any line matching `>+From ` (lossless unescaping).
    #[default]
    Mboxrd,
}

/// Configuration for [`MboxReader`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MboxReadConfig {
    /// Maximum raw normalized bytes buffered for a single message.
    pub max_message_bytes: usize,
    /// First IMAP-compatible synthetic UID assigned to parsed messages (must be >= 1).
    pub starting_uid: u32,
    /// `>From ` unescaping flavor applied to message bodies.
    pub flavor: MboxFlavor,
}

impl Default for MboxReadConfig {
    fn default() -> Self {
        Self {
            max_message_bytes: DEFAULT_MAX_MBOX_MESSAGE_BYTES,
            starting_uid: 1,
            flavor: MboxFlavor::Mboxrd,
        }
    }
}

/// A parsed message from a local `.mbox` or `.eml` archive alongside archive metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParsedArchiveMessage {
    pub message: ParsedInboundMessage,
    pub gmail_labels: Vec<String>,
    pub raw_size_bytes: usize,
}

impl ParsedArchiveMessage {
    /// Returns `true` if `X-Gmail-Labels` explicitly marks the message as `Unread`.
    #[must_use]
    pub fn has_unread_label(&self) -> bool {
        self.gmail_labels
            .iter()
            .any(|label| label.eq_ignore_ascii_case("Unread"))
    }

    /// Returns `true` if `X-Gmail-Labels` marks the message as `Starred`.
    #[must_use]
    pub fn has_starred_label(&self) -> bool {
        self.gmail_labels
            .iter()
            .any(|label| label.eq_ignore_ascii_case("Starred"))
    }
}

/// Streaming `.mbox` reader over any [`BufRead`] source.
///
/// INVARIANT: Memory usage is bounded to `config.max_message_bytes` plus a single
/// line buffer; multi-gigabyte `.mbox` files never load completely into RAM.
pub struct MboxReader<R: BufRead> {
    reader: R,
    config: MboxReadConfig,
    next_uid: u32,
    pending_from_line: Option<Vec<u8>>,
    started: bool,
    finished: bool,
    parser: StalwartMimeParser,
}

impl<R: BufRead> MboxReader<R> {
    /// Creates a new streaming `.mbox` reader with default configuration.
    pub fn new(reader: R) -> Result<Self, EngineError> {
        Self::with_config(reader, MboxReadConfig::default())
    }

    /// Creates a new streaming `.mbox` reader with explicit configuration.
    pub fn with_config(reader: R, config: MboxReadConfig) -> Result<Self, EngineError> {
        if config.max_message_bytes == 0 {
            return Err(EngineError::MimeParse(
                "mbox max_message_bytes must be greater than zero".into(),
            ));
        }
        if config.starting_uid == 0 {
            return Err(EngineError::MimeParse(
                "mbox starting_uid must be greater than zero".into(),
            ));
        }
        Ok(Self {
            reader,
            config,
            next_uid: config.starting_uid,
            pending_from_line: None,
            started: false,
            finished: false,
            parser: StalwartMimeParser::new(),
        })
    }

    /// Reads and parses the next message from the `.mbox` stream.
    ///
    /// Returns `Ok(None)` cleanly at EOF. Returns `Err` and halts further reads on
    /// malformed streams or messages exceeding `max_message_bytes`.
    pub fn next_message(&mut self) -> Result<Option<ParsedArchiveMessage>, EngineError> {
        if self.finished {
            return Ok(None);
        }

        if !self.started {
            self.started = true;
            match self.seek_initial_from_line()? {
                Some(line) => self.pending_from_line = Some(line),
                None => {
                    self.finished = true;
                    return Ok(None);
                }
            }
        }

        let Some(_envelope_line) = self.pending_from_line.take() else {
            self.finished = true;
            return Ok(None);
        };

        let mut raw_msg = Vec::new();
        let mut in_body = false;
        let mut last_line_was_empty = false;
        let mut line_buf = Vec::new();

        loop {
            line_buf.clear();
            let read_res = read_line_bounded(
                &mut self.reader,
                &mut line_buf,
                self.config.max_message_bytes,
            );
            let bytes_read = match read_res {
                Ok(n) => n,
                Err(e) => {
                    self.finished = true;
                    return Err(e);
                }
            };
            if bytes_read == 0 {
                self.finished = true;
                break;
            }

            let trimmed = strip_line_ending(&line_buf);
            if is_mbox_separator_line(trimmed) {
                self.pending_from_line = Some(line_buf.clone());
                break;
            }

            if !in_body {
                if trimmed.is_empty() {
                    in_body = true;
                    last_line_was_empty = false;
                    append_crlf_line(&mut raw_msg, b"", self.config.max_message_bytes)
                        .inspect_err(|_| {
                            self.finished = true;
                        })?;
                } else {
                    last_line_was_empty = false;
                    append_crlf_line(&mut raw_msg, trimmed, self.config.max_message_bytes)
                        .inspect_err(|_| {
                            self.finished = true;
                        })?;
                }
            } else {
                // Defer emitting a trailing empty line until we know the next line is not a `From ` delimiter.
                if last_line_was_empty {
                    append_crlf_line(&mut raw_msg, b"", self.config.max_message_bytes)
                        .inspect_err(|_| {
                            self.finished = true;
                        })?;
                }

                if trimmed.is_empty() {
                    last_line_was_empty = true;
                } else {
                    last_line_was_empty = false;
                    let unescaped = unescape_from_line(trimmed, self.config.flavor);
                    append_crlf_line(&mut raw_msg, unescaped, self.config.max_message_bytes)
                        .inspect_err(|_| {
                            self.finished = true;
                        })?;
                }
            }
        }

        if raw_msg.is_empty() {
            self.finished = true;
            return Err(EngineError::MimeParse(
                "mbox message contained no RFC 5322 content after From delimiter".into(),
            ));
        }

        let uid = self.next_uid;
        self.next_uid = self.next_uid.checked_add(1).ok_or_else(|| {
            self.finished = true;
            EngineError::MimeParse("mbox message UID space exhausted".into())
        })?;

        let raw_size_bytes = raw_msg.len();
        let gmail_labels = extract_gmail_labels(&raw_msg);
        let message = self.parser.parse_with_uid_and_modseq(&raw_msg, uid, None)?;

        Ok(Some(ParsedArchiveMessage {
            message,
            gmail_labels,
            raw_size_bytes,
        }))
    }

    fn seek_initial_from_line(&mut self) -> Result<Option<Vec<u8>>, EngineError> {
        let mut line_buf = Vec::new();
        loop {
            line_buf.clear();
            let n = read_line_bounded(
                &mut self.reader,
                &mut line_buf,
                self.config.max_message_bytes,
            )?;
            if n == 0 {
                return Ok(None);
            }
            let trimmed = strip_line_ending(&line_buf);
            if trimmed.is_empty() {
                continue;
            }
            if trimmed.starts_with(b"From ") {
                return Ok(Some(line_buf));
            }
            return Err(EngineError::MimeParse(
                "invalid mbox archive: expected 'From ' envelope delimiter".into(),
            ));
        }
    }
}

impl<R: BufRead> Iterator for MboxReader<R> {
    type Item = Result<ParsedArchiveMessage, EngineError>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.next_message() {
            Ok(Some(msg)) => Some(Ok(msg)),
            Ok(None) => None,
            Err(err) => Some(Err(err)),
        }
    }
}

/// Parses a single `.eml` message from raw bytes with CRLF normalization and a size cap.
pub fn parse_eml_bytes(
    raw_email: &[u8],
    uid: u32,
    max_message_bytes: usize,
) -> Result<ParsedArchiveMessage, EngineError> {
    if max_message_bytes == 0 {
        return Err(EngineError::MimeParse(
            "eml max_message_bytes must be greater than zero".into(),
        ));
    }
    if uid == 0 {
        return Err(EngineError::MimeParse(
            "eml uid must be greater than zero".into(),
        ));
    }
    if raw_email.is_empty() {
        return Err(EngineError::MimeParse("eml payload is empty".into()));
    }
    if raw_email.len() > max_message_bytes {
        return Err(EngineError::MimeParse(format!(
            "eml message exceeds maximum size of {max_message_bytes} bytes"
        )));
    }

    let normalized = normalize_crlf(raw_email, max_message_bytes)?;
    let gmail_labels = extract_gmail_labels(&normalized);
    let message = StalwartMimeParser::new().parse_with_uid_and_modseq(&normalized, uid, None)?;
    Ok(ParsedArchiveMessage {
        message,
        gmail_labels,
        raw_size_bytes: normalized.len(),
    })
}

/// Reads and parses a single `.eml` message from a stream without reading past `max_message_bytes`.
pub fn parse_eml_reader<R: Read>(
    reader: R,
    uid: u32,
    max_message_bytes: usize,
) -> Result<ParsedArchiveMessage, EngineError> {
    if max_message_bytes == 0 {
        return Err(EngineError::MimeParse(
            "eml max_message_bytes must be greater than zero".into(),
        ));
    }
    let limit = (max_message_bytes as u64).saturating_add(1);
    let mut buf = Vec::new();
    reader
        .take(limit)
        .read_to_end(&mut buf)
        .map_err(|e| EngineError::MimeParse(format!("failed to read eml stream: {e}")))?;
    parse_eml_bytes(&buf, uid, max_message_bytes)
}

/// Extracts and decodes `X-Gmail-Labels` values from normalized RFC 5322 headers.
#[must_use]
pub fn extract_gmail_labels(raw_email: &[u8]) -> Vec<String> {
    let header_slice = extract_header_slice(raw_email);
    let Some(parsed) = MessageParser::default().parse(header_slice) else {
        return Vec::new();
    };

    let mut labels = Vec::new();
    for header in parsed.headers() {
        if header.raw_name().eq_ignore_ascii_case("X-Gmail-Labels") {
            let raw_text = match header.value() {
                HeaderValue::Text(text) => text.to_string(),
                _ => String::from_utf8_lossy(header.raw_value()).into_owned(),
            };
            for token in split_csv_labels(&raw_text) {
                let decoded = decode_rfc2047_label(&token);
                let trimmed = decoded.trim();
                if !trimmed.is_empty() && !labels.iter().any(|existing| existing == trimmed) {
                    labels.push(trimmed.to_string());
                }
            }
        }
    }
    labels
}

fn decode_rfc2047_label(token: &str) -> String {
    if !token.contains("=?") {
        return token.to_string();
    }
    let synthetic = format!("Subject: {token}\r\n\r\n");
    MessageParser::default()
        .parse(synthetic.as_bytes())
        .and_then(|msg| msg.subject().map(str::to_string))
        .unwrap_or_else(|| token.to_string())
}

fn extract_header_slice(raw_email: &[u8]) -> &[u8] {
    if let Some(pos) = raw_email.windows(4).position(|w| w == b"\r\n\r\n") {
        return &raw_email[..pos + 4];
    }
    if let Some(pos) = raw_email.windows(2).position(|w| w == b"\n\n") {
        return &raw_email[..pos + 2];
    }
    raw_email
}

fn split_csv_labels(raw: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    let mut chars = raw.chars().peekable();

    while let Some(ch) = chars.next() {
        match ch {
            '"' => {
                if in_quotes {
                    if chars.peek() == Some(&'"') {
                        current.push('"');
                        let _ = chars.next();
                    } else {
                        in_quotes = false;
                    }
                } else {
                    in_quotes = true;
                }
            }
            '\\' if in_quotes => {
                if let Some(escaped) = chars.next() {
                    current.push(escaped);
                }
            }
            ',' if !in_quotes => {
                let trimmed = current.trim();
                if !trimmed.is_empty() {
                    out.push(trimmed.to_string());
                }
                current.clear();
            }
            _ => current.push(ch),
        }
    }

    let trimmed = current.trim();
    if !trimmed.is_empty() {
        out.push(trimmed.to_string());
    }
    out
}

fn read_line_bounded<R: BufRead>(
    reader: &mut R,
    out: &mut Vec<u8>,
    max_bytes: usize,
) -> Result<usize, EngineError> {
    let mut total = 0usize;
    loop {
        let available = reader
            .fill_buf()
            .map_err(|e| EngineError::MimeParse(format!("mbox stream read error: {e}")))?;
        if available.is_empty() {
            return Ok(total);
        }

        let (consume_len, found_newline) = match available.iter().position(|&b| b == b'\n') {
            Some(idx) => (idx + 1, true),
            None => (available.len(), false),
        };

        let next_total = total.checked_add(consume_len).ok_or_else(|| {
            EngineError::MimeParse("mbox line length overflowed maximum bounds".into())
        })?;
        // Allow up to max_bytes + 2 bytes for a single line before failing so CRLF normalization checks cleanly.
        if next_total > max_bytes.saturating_add(2) {
            return Err(EngineError::MimeParse(format!(
                "mbox message line exceeds maximum size of {max_bytes} bytes"
            )));
        }

        out.extend_from_slice(&available[..consume_len]);
        reader.consume(consume_len);
        total = next_total;

        if found_newline {
            return Ok(total);
        }
    }
}

fn strip_line_ending(line: &[u8]) -> &[u8] {
    let line = line.strip_suffix(b"\n").unwrap_or(line);
    line.strip_suffix(b"\r").unwrap_or(line)
}

fn append_crlf_line(
    out: &mut Vec<u8>,
    line: &[u8],
    max_message_bytes: usize,
) -> Result<(), EngineError> {
    let needed = line
        .len()
        .checked_add(2)
        .and_then(|n| out.len().checked_add(n))
        .ok_or_else(|| EngineError::MimeParse("mbox message size overflow".into()))?;
    if needed > max_message_bytes {
        return Err(EngineError::MimeParse(format!(
            "mbox message exceeds maximum size of {max_message_bytes} bytes"
        )));
    }
    out.extend_from_slice(line);
    out.extend_from_slice(b"\r\n");
    Ok(())
}

fn normalize_crlf(raw: &[u8], max_message_bytes: usize) -> Result<Vec<u8>, EngineError> {
    let mut out = Vec::with_capacity(raw.len());
    let mut i = 0;
    while i < raw.len() {
        if raw[i] == b'\r' {
            if raw.get(i + 1) == Some(&b'\n') {
                push_bounded(&mut out, b"\r\n", max_message_bytes)?;
                i += 2;
            } else {
                push_bounded(&mut out, b"\r\n", max_message_bytes)?;
                i += 1;
            }
        } else if raw[i] == b'\n' {
            push_bounded(&mut out, b"\r\n", max_message_bytes)?;
            i += 1;
        } else {
            let start = i;
            while i < raw.len() && raw[i] != b'\r' && raw[i] != b'\n' {
                i += 1;
            }
            push_bounded(&mut out, &raw[start..i], max_message_bytes)?;
        }
    }
    Ok(out)
}

fn push_bounded(out: &mut Vec<u8>, slice: &[u8], max_bytes: usize) -> Result<(), EngineError> {
    let next_len = out
        .len()
        .checked_add(slice.len())
        .ok_or_else(|| EngineError::MimeParse("message size overflow".into()))?;
    if next_len > max_bytes {
        return Err(EngineError::MimeParse(format!(
            "eml normalized message exceeds maximum size of {max_bytes} bytes"
        )));
    }
    out.extend_from_slice(slice);
    Ok(())
}

/// Checks whether a line (with line endings stripped) is an mbox `From ` envelope delimiter.
///
/// Matches standard Unix mbox, Thunderbird (`From - ...`), and Google Takeout (`From <id>@xxx ...`)
/// lines while avoiding false splits on unescaped prose starting with `"From "`.
fn is_mbox_separator_line(trimmed_line: &[u8]) -> bool {
    let Some(rest) = trimmed_line.strip_prefix(b"From ") else {
        return false;
    };
    let rest_str = match std::str::from_utf8(rest) {
        Ok(s) => s.trim(),
        Err(_) => return false,
    };
    let mut tokens = rest_str.split_ascii_whitespace();
    let Some(_sender_token) = tokens.next() else {
        return false;
    };
    let remainder: Vec<&str> = tokens.collect();
    if remainder.is_empty() {
        return false;
    }

    // Require at least one timestamp-like indicator (`HH:MM` or `YYYY-MM-DD`) in the date portion.
    remainder
        .iter()
        .any(|tok| looks_like_time(tok) || looks_like_iso_date(tok))
}

fn looks_like_time(token: &str) -> bool {
    let bytes = token.as_bytes();
    bytes.windows(5).any(|w| {
        w[0].is_ascii_digit()
            && w[1].is_ascii_digit()
            && w[2] == b':'
            && w[3].is_ascii_digit()
            && w[4].is_ascii_digit()
    })
}

fn looks_like_iso_date(token: &str) -> bool {
    let bytes = token.as_bytes();
    bytes.windows(10).any(|w| {
        w[0..4].iter().all(u8::is_ascii_digit)
            && w[4] == b'-'
            && w[5..7].iter().all(u8::is_ascii_digit)
            && w[7] == b'-'
            && w[8..10].iter().all(u8::is_ascii_digit)
    })
}

fn unescape_from_line(line: &[u8], flavor: MboxFlavor) -> &[u8] {
    match flavor {
        MboxFlavor::Mboxo => line
            .strip_prefix(b">")
            .filter(|rest| rest.starts_with(b"From "))
            .unwrap_or(line),
        MboxFlavor::Mboxrd => {
            let leading_gt = line.iter().take_while(|&&b| b == b'>').count();
            if leading_gt > 0 && line[leading_gt..].starts_with(b"From ") {
                &line[1..]
            } else {
                line
            }
        }
    }
}
