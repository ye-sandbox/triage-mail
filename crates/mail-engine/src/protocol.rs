// INVARIANT: IMAP protocol communication must safely parse untagged responses,
// quoted strings, and RFC 3501/7162 literals without buffering unconstrained streams.

use std::str;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

use crate::provider::ServerCapabilities;
use crate::EngineError;

/// Tagged response status code.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaggedStatus {
    Ok,
    No,
    Bad,
}

/// Status and metadata returned by `SELECT` or `EXAMINE`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FolderStatus {
    pub name: String,
    pub uidvalidity: u32,
    pub uidnext: u32,
    pub highest_modseq: Option<u64>,
    pub exists: u32,
}

/// Message UID and flag set returned by delta queries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UidFlags {
    pub uid: u32,
    pub flags: Vec<String>,
}

/// Real-time untagged notification received while a session is in RFC 2177 `IDLE` state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IdleNotification {
    /// Mailbox message count changed (`* <n> EXISTS`).
    Exists(u32),
    /// A message sequence number was expunged (`* <n> EXPUNGE`).
    Expunge(u32),
    /// Flags changed for a sequence number (`* <n> FETCH ...`).
    FlagsChanged(u32),
    /// Other untagged server line received during `IDLE`.
    Untagged(String),
}

/// Outcome of waiting on an active `IDLE` stream up to the heartbeat renewal deadline.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IdleWaitOutcome {
    /// Server pushed an untagged mailbox change notification.
    ServerEvent(IdleNotification),
    /// The 14-minute heartbeat timer elapsed without server events; caller should send `DONE` -> `IDLE`.
    HeartbeatTimeout,
    /// Server sent `* BYE` terminating the connection.
    ServerDisconnect(String),
}

/// Raw message parts fetched from the IMAP server.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RawFetchedMessage {
    pub seq: u32,
    pub uid: u32,
    pub modseq: Option<u64>,
    pub flags: Vec<String>,
    pub header_bytes: Vec<u8>,
    pub text_bytes: Vec<u8>,
    pub raw_payload: Vec<u8>,
    pub bodystructure: Option<String>,
    pub sections: std::collections::HashMap<String, Vec<u8>>,
}

/// Represents the completed execution of an IMAP tagged command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandResult {
    pub tag: String,
    pub status: TaggedStatus,
    pub message: String,
    pub untagged_lines: Vec<String>,
    pub fetched_messages: Vec<RawFetchedMessage>,
    pub folder_status: Option<FolderStatus>,
}

/// Stream-agnostic IMAP protocol session.
pub struct ImapSession<S> {
    stream: S,
    tag_sequence: u64,
    buffer: Vec<u8>,
}

impl<S: AsyncRead + AsyncWrite + Unpin + Send> ImapSession<S> {
    pub fn new(stream: S) -> Self {
        Self {
            stream,
            tag_sequence: 1,
            buffer: Vec::with_capacity(8192),
        }
    }

    /// Generates the next sequential command tag (e.g. `A0001`).
    fn next_tag(&mut self) -> String {
        let tag = format!("A{:04}", self.tag_sequence);
        self.tag_sequence += 1;
        tag
    }

    /// Reads bytes from stream into internal buffer until finding CRLF (`\r\n`).
    async fn read_crlf_line(&mut self) -> Result<String, EngineError> {
        loop {
            if let Some(pos) = self.buffer.windows(2).position(|w| w == b"\r\n") {
                let line_bytes: Vec<u8> = self.buffer.drain(..pos + 2).collect();
                let line_str = str::from_utf8(&line_bytes[..pos])
                    .map_err(|e| {
                        EngineError::ImapProtocol(format!("invalid UTF-8 in IMAP line: {e}"))
                    })?
                    .to_string();
                return Ok(line_str);
            }

            let mut chunk = [0u8; 4096];
            let n = self.stream.read(&mut chunk).await.map_err(|e| {
                EngineError::ImapProtocol(format!("failed reading from IMAP socket: {e}"))
            })?;
            if n == 0 {
                return Err(EngineError::ImapProtocol(
                    "unexpected EOF from IMAP server".to_string(),
                ));
            }
            self.buffer.extend_from_slice(&chunk[..n]);
        }
    }

    /// Reads exactly `size` literal bytes from the stream.
    async fn read_exact_bytes(&mut self, size: usize) -> Result<Vec<u8>, EngineError> {
        while self.buffer.len() < size {
            let mut chunk = [0u8; 4096];
            let n = self.stream.read(&mut chunk).await.map_err(|e| {
                EngineError::ImapProtocol(format!("failed reading literal bytes: {e}"))
            })?;
            if n == 0 {
                return Err(EngineError::ImapProtocol(
                    "unexpected EOF while reading IMAP literal".to_string(),
                ));
            }
            self.buffer.extend_from_slice(&chunk[..n]);
        }

        let literal = self.buffer.drain(..size).collect();
        Ok(literal)
    }

    /// Reads initial server greeting line (e.g. `* OK ...`).
    pub async fn read_greeting(&mut self) -> Result<String, EngineError> {
        self.read_crlf_line().await
    }

    /// Executes an IMAP command and reads all untagged responses until the matching tagged status line.
    pub async fn execute_command(&mut self, command: &str) -> Result<CommandResult, EngineError> {
        let tag = self.next_tag();
        let wire_command = format!("{} {}\r\n", tag, command);

        self.stream
            .write_all(wire_command.as_bytes())
            .await
            .map_err(|e| {
                EngineError::ImapProtocol(format!("failed writing command to socket: {e}"))
            })?;
        self.stream
            .flush()
            .await
            .map_err(|e| EngineError::ImapProtocol(format!("failed flushing IMAP socket: {e}")))?;

        let mut untagged_lines = Vec::new();
        let mut fetched_messages = Vec::new();
        let mut folder_status = FolderStatus::default();

        loop {
            let line = self.read_crlf_line().await?;

            if line.starts_with(&tag) {
                // Tagged completion line: e.g. "A0001 OK ...", "A0001 NO ...", "A0001 BAD ..."
                let parts: Vec<&str> = line.splitn(3, ' ').collect();
                if parts.len() < 2 {
                    return Err(EngineError::ImapProtocol(format!(
                        "malformed tagged response: {line}"
                    )));
                }

                let status = match parts[1] {
                    "OK" => TaggedStatus::Ok,
                    "NO" => TaggedStatus::No,
                    "BAD" => TaggedStatus::Bad,
                    other => {
                        return Err(EngineError::ImapProtocol(format!(
                            "unrecognized tagged status: {other}"
                        )))
                    }
                };

                let msg = if parts.len() >= 3 {
                    parts[2].to_string()
                } else {
                    String::new()
                };

                return Ok(CommandResult {
                    tag,
                    status,
                    message: msg,
                    untagged_lines,
                    fetched_messages,
                    folder_status: if folder_status.uidvalidity > 0 || folder_status.exists > 0 {
                        Some(folder_status)
                    } else {
                        None
                    },
                });
            } else if line.starts_with('*') {
                // Untagged line
                if line.contains("EXISTS") {
                    if let Some(num_str) = line.split_whitespace().nth(1) {
                        if let Ok(exists) = num_str.parse::<u32>() {
                            folder_status.exists = exists;
                        }
                    }
                }

                if line.contains("UIDVALIDITY") {
                    if let Some(val) = extract_bracket_token(&line, "UIDVALIDITY") {
                        if let Ok(num) = val.parse::<u32>() {
                            folder_status.uidvalidity = num;
                        }
                    }
                }

                if line.contains("UIDNEXT") {
                    if let Some(val) = extract_bracket_token(&line, "UIDNEXT") {
                        if let Ok(num) = val.parse::<u32>() {
                            folder_status.uidnext = num;
                        }
                    }
                }

                if line.contains("HIGHESTMODSEQ") {
                    if let Some(val) = extract_bracket_token(&line, "HIGHESTMODSEQ") {
                        if let Ok(num) = val.parse::<u64>() {
                            folder_status.highest_modseq = Some(num);
                        }
                    }
                }

                if line.contains("FETCH") {
                    let fetched = self.parse_fetch_response_line(&line).await?;
                    fetched_messages.push(fetched);
                } else {
                    untagged_lines.push(line);
                }
            } else {
                untagged_lines.push(line);
            }
        }
    }

    /// Parses an untagged `* <seq> FETCH (...)` line, reading all literal streams (RFC 3501 Section 4.3).
    async fn parse_fetch_response_line(
        &mut self,
        line: &str,
    ) -> Result<RawFetchedMessage, EngineError> {
        let mut msg = RawFetchedMessage::default();
        let mut current_chunk = line.to_string();

        loop {
            // Extract metadata from current chunk
            if msg.seq == 0 {
                if let Some(seq_str) = current_chunk.split_whitespace().nth(1) {
                    msg.seq = seq_str.parse::<u32>().unwrap_or(0);
                }
            }

            if msg.uid == 0 {
                if let Some(uid_pos) = current_chunk.find("UID ") {
                    let remainder = &current_chunk[uid_pos + 4..];
                    let token = remainder
                        .split(|c: char| !c.is_ascii_digit())
                        .next()
                        .unwrap_or_default();
                    if let Ok(uid) = token.parse::<u32>() {
                        msg.uid = uid;
                    }
                }
            }

            if msg.modseq.is_none() {
                if let Some(mod_pos) = current_chunk.find("MODSEQ (") {
                    let remainder = &current_chunk[mod_pos + 8..];
                    let token = remainder
                        .split(|c: char| !c.is_ascii_digit())
                        .next()
                        .unwrap_or_default();
                    if let Ok(modseq) = token.parse::<u64>() {
                        msg.modseq = Some(modseq);
                    }
                }
            }

            if msg.flags.is_empty() {
                if let Some(flags_pos) = current_chunk.find("FLAGS (") {
                    let remainder = &current_chunk[flags_pos + 7..];
                    if let Some(closing) = remainder.find(')') {
                        let flags_str = &remainder[..closing];
                        msg.flags = flags_str
                            .split_whitespace()
                            .map(|s| s.to_string())
                            .collect();
                    }
                }
            }

            if msg.bodystructure.is_none() {
                if let Some(bs) = crate::bodystructure::extract_bodystructure_sexpr(&current_chunk)
                {
                    msg.bodystructure = Some(bs.to_string());
                }
            }

            if let Some(literal_size) = extract_trailing_literal_size(&current_chunk) {
                let literal = self.read_exact_bytes(literal_size).await?;

                if let Some(section) = extract_body_section_name(&current_chunk) {
                    let section_upper = section.to_ascii_uppercase();
                    if section_upper == "HEADER" {
                        msg.header_bytes = literal;
                    } else if section_upper == "TEXT" {
                        msg.text_bytes = literal;
                    } else {
                        msg.sections.insert(section, literal);
                    }
                } else if current_chunk.to_ascii_uppercase().contains("BODYSTRUCTURE") {
                    if let Ok(bs_str) = std::str::from_utf8(&literal) {
                        msg.bodystructure = Some(bs_str.to_string());
                    }
                } else if msg.raw_payload.is_empty() {
                    msg.raw_payload = literal;
                }

                // Read next line chunk continuation
                current_chunk = self.read_crlf_line().await?;
            } else {
                break;
            }
        }

        Ok(msg)
    }

    /// Queries server capabilities via `CAPABILITY`.
    pub async fn capability(&mut self) -> Result<ServerCapabilities, EngineError> {
        let res = self.execute_command("CAPABILITY").await?;
        if res.status != TaggedStatus::Ok {
            return Err(EngineError::ImapProtocol(format!(
                "CAPABILITY failed: {}",
                res.message
            )));
        }

        for line in &res.untagged_lines {
            if line.contains("CAPABILITY") {
                return Ok(ServerCapabilities::from_capability_line(line));
            }
        }

        Ok(ServerCapabilities::default())
    }

    /// Authenticates using standard IMAP `LOGIN`.
    pub async fn login(&mut self, user: &str, pass: &str) -> Result<(), EngineError> {
        let escaped_user = escape_imap_string(user);
        let escaped_pass = escape_imap_string(pass);
        let cmd = format!("LOGIN \"{}\" \"{}\"", escaped_user, escaped_pass);

        let res = self.execute_command(&cmd).await?;
        if res.status != TaggedStatus::Ok {
            return Err(EngineError::ImapProtocol(format!(
                "LOGIN authentication failed: {}",
                res.message
            )));
        }
        Ok(())
    }

    /// Authenticates using SASL XOAUTH2.
    pub async fn authenticate_xoauth2(
        &mut self,
        user: &str,
        access_token: &str,
    ) -> Result<(), EngineError> {
        let sasl_string = crate::auth::build_xoauth2_sasl_string(user, access_token);
        let cmd = format!("AUTHENTICATE XOAUTH2 {}", sasl_string);

        let res = self.execute_command(&cmd).await?;
        if res.status != TaggedStatus::Ok {
            return Err(EngineError::ImapProtocol(format!(
                "XOAUTH2 authentication failed: {}",
                res.message
            )));
        }
        Ok(())
    }

    /// Selects a folder in READ-WRITE mode and parses status attributes.
    pub async fn select_folder(&mut self, folder: &str) -> Result<FolderStatus, EngineError> {
        let escaped_folder = escape_imap_string(folder);
        let cmd = format!("SELECT \"{}\"", escaped_folder);

        let res = self.execute_command(&cmd).await?;
        if res.status != TaggedStatus::Ok {
            return Err(EngineError::ImapProtocol(format!(
                "SELECT folder `{folder}` failed: {}",
                res.message
            )));
        }

        let mut status = res.folder_status.unwrap_or_default();
        status.name = folder.to_string();
        Ok(status)
    }

    /// Searches for message UIDs matching a search criterion (e.g. `UID 101:*`).
    pub async fn search_uids(&mut self, query: &str) -> Result<Vec<u32>, EngineError> {
        let cmd = format!("UID SEARCH {}", query);
        let res = self.execute_command(&cmd).await?;
        if res.status != TaggedStatus::Ok {
            return Err(EngineError::ImapProtocol(format!(
                "UID SEARCH failed: {}",
                res.message
            )));
        }

        let mut uids = Vec::new();
        for line in &res.untagged_lines {
            if let Some(rest) = line.strip_prefix("* SEARCH") {
                for token in rest.split_whitespace() {
                    if let Ok(uid) = token.parse::<u32>() {
                        uids.push(uid);
                    }
                }
            }
        }
        uids.sort_unstable();
        uids.dedup();
        Ok(uids)
    }

    /// Fetches message structure, headers, flags, and modseq for a set of UIDs.
    pub async fn fetch_uids_structure_and_headers(
        &mut self,
        uids: &[u32],
    ) -> Result<Vec<RawFetchedMessage>, EngineError> {
        if uids.is_empty() {
            return Ok(Vec::new());
        }
        let seq_set = format_sequence_set(uids);
        let cmd = format!(
            "UID FETCH {} (UID MODSEQ FLAGS BODYSTRUCTURE BODY.PEEK[HEADER])",
            seq_set
        );
        let res = self.execute_command(&cmd).await?;
        if res.status != TaggedStatus::Ok {
            return Err(EngineError::ImapProtocol(format!(
                "UID FETCH structure and headers failed: {}",
                res.message
            )));
        }
        Ok(res.fetched_messages)
    }

    /// Fetches selective MIME/body sections for a set of UIDs.
    pub async fn fetch_uids_sections(
        &mut self,
        uids: &[u32],
        sections: &[String],
    ) -> Result<Vec<RawFetchedMessage>, EngineError> {
        if uids.is_empty() || sections.is_empty() {
            return Ok(Vec::new());
        }
        let seq_set = format_sequence_set(uids);
        let items = sections.join(" ");
        let cmd = format!("UID FETCH {} (UID {})", seq_set, items);
        let res = self.execute_command(&cmd).await?;
        if res.status != TaggedStatus::Ok {
            return Err(EngineError::ImapProtocol(format!(
                "UID FETCH sections failed: {}",
                res.message
            )));
        }
        Ok(res.fetched_messages)
    }

    /// Fetches messages with UID greater than `last_uid` using the two-phase Lazy Attachment protocol.
    ///
    /// INVARIANT: Never fetch BODY.PEEK[TEXT] or RFC822 during bulk sync because multipart TEXT contains
    /// raw binary attachments. Fetch BODYSTRUCTURE first, then fetch exclusively selective non-attachment
    /// text/html sections (BODY.PEEK[<section>]) plus headers/flags/modseq.
    pub async fn fetch_new_messages(
        &mut self,
        last_uid: u32,
    ) -> Result<Vec<RawFetchedMessage>, EngineError> {
        self.fetch_new_messages_two_phase(last_uid, crate::DEFAULT_SYNC_UID_BATCH_SIZE)
            .await
    }

    /// Fetches new messages strictly using the two-phase Lazy Attachment protocol:
    /// Phase 1: Discovers candidate UIDs and fetches BODYSTRUCTURE + HEADER (never BODY.PEEK[TEXT]).
    /// Phase 2: Fetches exclusively selective text/plain and text/html leaf sections.
    pub async fn fetch_new_messages_two_phase(
        &mut self,
        last_uid: u32,
        batch_size: u32,
    ) -> Result<Vec<RawFetchedMessage>, EngineError> {
        let search_query = format!("UID {}:*", last_uid.saturating_add(1));
        let candidate_uids = match self.search_uids(&search_query).await {
            Ok(uids) => {
                let filtered: Vec<u32> = uids.into_iter().filter(|&u| u > last_uid).collect();
                filtered
            }
            Err(_) => Vec::new(),
        };

        let mut messages = if !candidate_uids.is_empty() {
            let limit = (batch_size as usize).min(candidate_uids.len());
            let batch_uids = &candidate_uids[..limit];
            self.fetch_uids_structure_and_headers(batch_uids).await?
        } else {
            // Direct bounded range query if candidate search was empty or failed
            let start = last_uid.saturating_add(1);
            let range = format!("{}:*", start);
            let mut msgs = self
                .execute_command(&format!(
                    "UID FETCH {} (UID MODSEQ FLAGS BODYSTRUCTURE BODY.PEEK[HEADER])",
                    range
                ))
                .await?
                .fetched_messages;
            msgs.retain(|m| m.uid > last_uid);
            msgs.sort_by_key(|m| m.uid);
            if msgs.len() > batch_size as usize {
                msgs.truncate(batch_size as usize);
            }
            msgs
        };

        messages.retain(|m| m.uid > last_uid);
        messages.sort_by_key(|m| m.uid);

        self.fetch_phase2_for_messages(messages).await
    }

    async fn fetch_phase2_for_messages(
        &mut self,
        mut messages: Vec<RawFetchedMessage>,
    ) -> Result<Vec<RawFetchedMessage>, EngineError> {
        if messages.is_empty() {
            return Ok(messages);
        }

        let mut section_to_uids: std::collections::HashMap<Vec<String>, Vec<u32>> =
            std::collections::HashMap::new();

        for msg in &messages {
            let mut needed_sections = Vec::new();
            if let Some(bs_str) = &msg.bodystructure {
                if let Ok(structure) = crate::bodystructure::parse_bodystructure_str(bs_str) {
                    let meta = crate::bodystructure::extract_body_structure_metadata(&structure);
                    if meta.is_single_part {
                        if !meta.text_sections.is_empty() {
                            needed_sections.push("BODY.PEEK[1]".to_string());
                        }
                    } else {
                        for sec in meta.text_sections {
                            needed_sections.push(format!("BODY.PEEK[{}.MIME]", sec.section));
                            needed_sections.push(format!("BODY.PEEK[{}]", sec.section));
                        }
                    }
                }
            } else {
                needed_sections.push("BODY.PEEK[1]".to_string());
            }

            if !needed_sections.is_empty() {
                section_to_uids
                    .entry(needed_sections)
                    .or_default()
                    .push(msg.uid);
            }
        }

        for (sections, uids) in section_to_uids {
            let fetched_sections = self.fetch_uids_sections(&uids, &sections).await?;
            for fetched in fetched_sections {
                if let Some(target) = messages.iter_mut().find(|m| m.uid == fetched.uid) {
                    for (k, v) in fetched.sections {
                        target.sections.insert(k, v);
                    }
                    if target.raw_payload.is_empty() && !fetched.raw_payload.is_empty() {
                        target.raw_payload = fetched.raw_payload;
                    }
                }
            }
        }

        Ok(messages)
    }

    /// Fetches changed message flags since `modseq` using RFC 7162 CONDSTORE.
    pub async fn fetch_flags_changed_since(
        &mut self,
        modseq: u64,
    ) -> Result<Vec<UidFlags>, EngineError> {
        let cmd = format!("UID FETCH 1:* (UID FLAGS) (CHANGEDSINCE {})", modseq);
        let res = self.execute_command(&cmd).await?;
        if res.status != TaggedStatus::Ok {
            return Err(EngineError::ImapProtocol(format!(
                "CHANGEDSINCE FETCH failed: {}",
                res.message
            )));
        }

        let flags_list = res
            .fetched_messages
            .into_iter()
            .map(|m| UidFlags {
                uid: m.uid,
                flags: m.flags,
            })
            .collect();

        Ok(flags_list)
    }

    /// Fallback reconciliation: fetches all message UIDs and flags without CONDSTORE.
    pub async fn fetch_all_uid_flags(&mut self) -> Result<Vec<UidFlags>, EngineError> {
        let res = self.execute_command("UID FETCH 1:* (UID FLAGS)").await?;
        if res.status != TaggedStatus::Ok {
            return Err(EngineError::ImapProtocol(format!(
                "Fallback UID FETCH 1:* failed: {}",
                res.message
            )));
        }

        let flags_list = res
            .fetched_messages
            .into_iter()
            .map(|m| UidFlags {
                uid: m.uid,
                flags: m.flags,
            })
            .collect();

        Ok(flags_list)
    }

    /// Fetches a single binary attachment part on demand via `IMAP BODY.PEEK[part_id]`.
    pub async fn fetch_attachment_bytes(
        &mut self,
        uid: u32,
        part_id: &str,
    ) -> Result<Vec<u8>, EngineError> {
        let cmd = format!("UID FETCH {} (BODY.PEEK[{}])", uid, part_id);
        let res = self.execute_command(&cmd).await?;
        if res.status != TaggedStatus::Ok {
            return Err(EngineError::ImapProtocol(format!(
                "fetching attachment part failed: {}",
                res.message
            )));
        }

        let bytes = res
            .fetched_messages
            .into_iter()
            .find(|m| m.uid == uid)
            .map(|mut m| m.sections.remove(part_id).unwrap_or(m.raw_payload))
            .unwrap_or_default();

        Ok(bytes)
    }

    /// Enters RFC 2177 `IDLE` mode and returns the active command tag once `+` continuation is received.
    pub async fn enter_idle(&mut self) -> Result<String, EngineError> {
        let tag = self.next_tag();
        let wire_command = format!("{} IDLE\r\n", tag);

        self.stream
            .write_all(wire_command.as_bytes())
            .await
            .map_err(|e| EngineError::ImapProtocol(format!("failed sending IDLE command: {e}")))?;
        self.stream
            .flush()
            .await
            .map_err(|e| EngineError::ImapProtocol(format!("failed flushing IDLE command: {e}")))?;

        loop {
            let line = self.read_crlf_line().await?;
            if line.starts_with('+') {
                return Ok(tag);
            }
            if line.starts_with(&tag) {
                return Err(EngineError::ImapProtocol(format!(
                    "server rejected IDLE command: {line}"
                )));
            }
        }
    }

    /// Waits for an untagged server notification during `IDLE` or returns `HeartbeatTimeout`
    /// when `heartbeat_timeout` elapses (default 14 minutes per `k-002` NAT/Outlook invariant).
    pub async fn wait_idle_event_or_timeout(
        &mut self,
        heartbeat_timeout: std::time::Duration,
    ) -> Result<IdleWaitOutcome, EngineError> {
        match tokio::time::timeout(heartbeat_timeout, self.read_crlf_line()).await {
            Err(_elapsed) => Ok(IdleWaitOutcome::HeartbeatTimeout),
            Ok(Err(e)) => Err(e),
            Ok(Ok(line)) => Ok(parse_idle_untagged_line(&line)),
        }
    }

    /// Sends `DONE\r\n` to terminate RFC 2177 `IDLE` mode and waits for the tagged `OK` completion.
    pub async fn exit_idle(&mut self, idle_tag: &str) -> Result<(), EngineError> {
        self.stream
            .write_all(b"DONE\r\n")
            .await
            .map_err(|e| EngineError::ImapProtocol(format!("failed sending DONE command: {e}")))?;
        self.stream
            .flush()
            .await
            .map_err(|e| EngineError::ImapProtocol(format!("failed flushing DONE command: {e}")))?;

        loop {
            let line = self.read_crlf_line().await?;
            if line.starts_with(idle_tag) {
                let parts: Vec<&str> = line.splitn(3, ' ').collect();
                if parts.len() >= 2 && parts[1] == "OK" {
                    return Ok(());
                }
                return Err(EngineError::ImapProtocol(format!(
                    "IDLE termination returned non-OK status: {line}"
                )));
            }
        }
    }
}

/// Parses an untagged server line emitted while in `IDLE` mode.
pub fn parse_idle_untagged_line(line: &str) -> IdleWaitOutcome {
    let trimmed = line.trim();
    if trimmed.starts_with("* BYE") || trimmed.starts_with("* bye") {
        return IdleWaitOutcome::ServerDisconnect(trimmed.to_string());
    }

    if let Some(rest) = trimmed.strip_prefix("* ") {
        let mut parts = rest.split_whitespace();
        if let (Some(first), Some(second)) = (parts.next(), parts.next()) {
            if let Ok(seq) = first.parse::<u32>() {
                match second.to_ascii_uppercase().as_str() {
                    "EXISTS" => return IdleWaitOutcome::ServerEvent(IdleNotification::Exists(seq)),
                    "EXPUNGE" => {
                        return IdleWaitOutcome::ServerEvent(IdleNotification::Expunge(seq))
                    }
                    "FETCH" => {
                        return IdleWaitOutcome::ServerEvent(IdleNotification::FlagsChanged(seq))
                    }
                    _ => {}
                }
            }
        }
    }

    IdleWaitOutcome::ServerEvent(IdleNotification::Untagged(trimmed.to_string()))
}

/// Helper extracting trailing literal length `{<size>}` if line ends with a literal marker.
fn extract_trailing_literal_size(s: &str) -> Option<usize> {
    let trimmed = s.trim_end();
    if trimmed.ends_with('}') {
        if let Some(brace_start) = trimmed.rfind('{') {
            let size_str = &trimmed[brace_start + 1..trimmed.len() - 1];
            return size_str.parse::<usize>().ok();
        }
    }
    None
}

/// Helper extracting bracketed tokens like `[UIDVALIDITY 12345]`.
fn extract_bracket_token(line: &str, token_name: &str) -> Option<String> {
    if let Some(start) = line.find(token_name) {
        let remainder = &line[start + token_name.len()..];
        let trimmed = remainder.trim_start();
        let token = trimmed.split([']', ' ', '\r', '\n']).next()?;
        if !token.is_empty() {
            return Some(token.to_string());
        }
    }
    None
}

/// Formats a list of UIDs into a valid RFC 3501 sequence-set.
pub fn format_sequence_set(uids: &[u32]) -> String {
    if uids.is_empty() {
        return String::new();
    }
    if uids.len() == 1 {
        return uids[0].to_string();
    }
    let is_contiguous = uids.windows(2).all(|w| w[1] == w[0].saturating_add(1));
    if is_contiguous {
        format!("{}:{}", uids[0], uids[uids.len() - 1])
    } else {
        uids.iter()
            .map(|u| u.to_string())
            .collect::<Vec<_>>()
            .join(",")
    }
}

/// Extracts section tag from `BODY[...]` or `BODY.PEEK[...]`.
pub fn extract_body_section_name(chunk: &str) -> Option<String> {
    let pos = chunk.find("BODY[").or_else(|| chunk.find("BODY.PEEK["))?;
    let start = chunk[pos..].find('[')? + pos + 1;
    let end = chunk[start..].find(']')? + start;
    Some(chunk[start..end].to_string())
}

/// Escapes double-quotes and backslashes in IMAP strings.
pub fn escape_imap_string(input: &str) -> String {
    input.replace('\\', "\\\\").replace('"', "\\\"")
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::duplex;

    #[tokio::test]
    async fn executes_login_and_parses_status() {
        let (client_stream, mut server_stream) = duplex(4096);
        let mut session = ImapSession::new(client_stream);

        tokio::spawn(async move {
            let mut buf = [0u8; 1024];
            let n = server_stream.read(&mut buf).await.unwrap();
            let req = String::from_utf8_lossy(&buf[..n]);
            assert!(req.contains("LOGIN \"user@example.com\" \"pass123\""));

            let resp = "A0001 OK LOGIN completed\r\n";
            server_stream.write_all(resp.as_bytes()).await.unwrap();
        });

        let res = session.login("user@example.com", "pass123").await;
        assert!(res.is_ok());
    }

    #[tokio::test]
    async fn parses_select_folder_with_condstore_metadata() {
        let (client_stream, mut server_stream) = duplex(4096);
        let mut session = ImapSession::new(client_stream);

        tokio::spawn(async move {
            let mut buf = [0u8; 1024];
            let _ = server_stream.read(&mut buf).await.unwrap();

            let resp = "* 42 EXISTS\r\n* OK [UIDVALIDITY 3001] UIDs valid\r\n* OK [UIDNEXT 501] Next UID\r\n* OK [HIGHESTMODSEQ 9005] Modseq\r\nA0001 OK [READ-WRITE] SELECT completed\r\n";
            server_stream.write_all(resp.as_bytes()).await.unwrap();
        });

        let status = session.select_folder("INBOX").await.unwrap();
        assert_eq!(status.exists, 42);
        assert_eq!(status.uidvalidity, 3001);
        assert_eq!(status.uidnext, 501);
        assert_eq!(status.highest_modseq, Some(9005));
    }

    #[test]
    fn formats_sequence_sets_correctly() {
        assert_eq!(format_sequence_set(&[]), "");
        assert_eq!(format_sequence_set(&[42]), "42");
        assert_eq!(format_sequence_set(&[1, 2, 3, 4, 5]), "1:5");
        assert_eq!(format_sequence_set(&[1, 3, 5]), "1,3,5");
    }

    #[test]
    fn extracts_body_section_names_correctly() {
        assert_eq!(
            extract_body_section_name("* 1 FETCH (BODY[HEADER] {120}"),
            Some("HEADER".to_string())
        );
        assert_eq!(
            extract_body_section_name("* 1 FETCH (BODY[1.MIME] {80}"),
            Some("1.MIME".to_string())
        );
        assert_eq!(
            extract_body_section_name("* 1 FETCH (BODY.PEEK[1.1] {500}"),
            Some("1.1".to_string())
        );
        assert_eq!(
            extract_body_section_name("* 1 FETCH (BODY[2] {1000}"),
            Some("2".to_string())
        );
        assert_eq!(
            extract_body_section_name("* 1 FETCH (FLAGS (\\Seen))"),
            None
        );
    }
}
