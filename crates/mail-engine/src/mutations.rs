//! Headless, UIDVALIDITY-checked IMAP mutations and lazy attachment access.
use tokio::io::{AsyncRead, AsyncWrite};

use crate::protocol::{escape_imap_string, CommandResult, ImapSession, TaggedStatus};
use crate::provider::{classify_folder, ProviderKind, ServerCapabilities, SpecialFolderRole};
use crate::EngineError;

#[derive(Debug, thiserror::Error)]
pub enum MutationError {
    #[error(transparent)]
    Engine(#[from] EngineError),
    #[error("message UIDVALIDITY is stale")]
    StaleIdentity,
    #[error("server operation unsupported")]
    Unsupported,
    #[error("invalid mutation request")]
    InvalidRequest,
}

#[derive(Debug, Clone, Copy)]
pub enum ImapMutation {
    Read(bool),
    MoveTo(SpecialFolderRole),
}

pub struct ImapMutationRequest<'a> {
    pub folder: &'a str,
    pub uid: u32,
    pub uidvalidity: u32,
    pub action: ImapMutation,
}

pub enum MutationOutcome {
    FlagsUpdated,
    Moved {
        folder: String,
        uid: Option<u32>,
        uidvalidity: Option<u32>,
    },
}

async fn select_checked<S: AsyncRead + AsyncWrite + Unpin + Send>(
    session: &mut ImapSession<S>,
    folder: &str,
    uid: u32,
    uidvalidity: u32,
) -> Result<(), MutationError> {
    if uid == 0 || folder.is_empty() || folder.chars().any(char::is_control) {
        return Err(MutationError::InvalidRequest);
    }
    let status = session.select_folder(folder).await?;
    // INVARIANT: A queued UID must never affect a different UIDVALIDITY generation.
    if uidvalidity == 0 || uidvalidity != status.uidvalidity {
        return Err(MutationError::StaleIdentity);
    }
    Ok(())
}

pub async fn fetch_attachment_checked<S: AsyncRead + AsyncWrite + Unpin + Send>(
    session: &mut ImapSession<S>,
    folder: &str,
    uid: u32,
    part: &str,
    uidvalidity: u32,
) -> Result<Vec<u8>, MutationError> {
    if part.is_empty()
        || !part
            .split('.')
            .all(|n| n.parse::<u32>().is_ok_and(|n| n > 0))
    {
        return Err(MutationError::InvalidRequest);
    }
    select_checked(session, folder, uid, uidvalidity).await?;
    // INVARIANT: Never fetch full RFC822; only the explicitly requested BODY.PEEK section.
    Ok(session.fetch_attachment_bytes(uid, part).await?)
}

pub async fn execute_imap_mutation<S: AsyncRead + AsyncWrite + Unpin + Send>(
    session: &mut ImapSession<S>,
    caps: &ServerCapabilities,
    provider: ProviderKind,
    request: &ImapMutationRequest<'_>,
) -> Result<MutationOutcome, MutationError> {
    select_checked(session, request.folder, request.uid, request.uidvalidity).await?;
    match request.action {
        ImapMutation::Read(read) => {
            let operator = if read { '+' } else { '-' };
            checked_command(
                session,
                &format!("UID STORE {} {operator}FLAGS.SILENT (\\Seen)", request.uid),
            )
            .await?;
            Ok(MutationOutcome::FlagsUpdated)
        }
        ImapMutation::MoveTo(role) => {
            move_to_special_folder(session, caps, provider, request, role).await
        }
    }
}

async fn checked_command<S: AsyncRead + AsyncWrite + Unpin + Send>(
    session: &mut ImapSession<S>,
    command: &str,
) -> Result<CommandResult, MutationError> {
    let result = session.execute_command(command).await?;
    if result.status != TaggedStatus::Ok {
        return Err(EngineError::ImapProtocol("mutation command rejected".into()).into());
    }
    Ok(result)
}

async fn move_to_special_folder<S: AsyncRead + AsyncWrite + Unpin + Send>(
    session: &mut ImapSession<S>,
    caps: &ServerCapabilities,
    provider: ProviderKind,
    request: &ImapMutationRequest<'_>,
    role: SpecialFolderRole,
) -> Result<MutationOutcome, MutationError> {
    // INVARIANT: No blanket EXPUNGE fallback: it could delete unrelated user messages.
    if !caps.move_cmd {
        return Err(MutationError::Unsupported);
    }
    let list = checked_command(session, "LIST \"\" \"*\"").await?;
    let folder = list
        .untagged_lines
        .iter()
        .find_map(|line| special_folder(line, role, provider == ProviderKind::Gmail))
        .ok_or(MutationError::Unsupported)?;
    if folder == request.folder {
        return Ok(MutationOutcome::Moved {
            folder,
            uid: Some(request.uid),
            uidvalidity: Some(request.uidvalidity),
        });
    }
    let result = checked_command(
        session,
        &format!(
            "UID MOVE {} \"{}\"",
            request.uid,
            escape_imap_string(&folder)
        ),
    )
    .await?;
    // RFC 6851 COPYUID can arrive in an untagged OK before the tagged completion.
    let mapping = result
        .untagged_lines
        .iter()
        .chain(std::iter::once(&result.message))
        .find_map(|line| copy_uid(line, request.uid));
    Ok(MutationOutcome::Moved {
        folder,
        uid: mapping.map(|(_, uid)| uid),
        uidvalidity: mapping.map(|(validity, _)| validity),
    })
}

fn copy_uid(message: &str, source: u32) -> Option<(u32, u32)> {
    let text = message.split_once("[COPYUID ")?.1.split_once(']')?.0;
    let mut parts = text.split_whitespace();
    let validity = parts.next()?.parse().ok()?;
    if parts.next()?.parse::<u32>().ok()? != source {
        return None;
    }
    let uid = parts.next()?.parse().ok()?;
    Some((validity, uid))
}

/// Handles quoted mailbox names safely; unsupported literal LIST names fail closed.
fn special_folder(line: &str, role: SpecialFolderRole, gmail: bool) -> Option<String> {
    let (_, rest) = line.strip_prefix("* LIST ")?.split_once('(')?;
    let (attrs, rest) = rest.split_once(')')?;
    let attrs: Vec<_> = attrs.split_whitespace().collect();
    let mut tokens = rest.trim().splitn(2, ' ');
    tokens.next()?; // hierarchy delimiter
    let raw = tokens.next()?.trim();
    if raw.starts_with('{') {
        return None;
    }
    let name = if raw.starts_with('"') && raw.ends_with('"') {
        raw[1..raw.len() - 1]
            .replace("\\\"", "\"")
            .replace("\\\\", "\\")
    } else {
        raw.to_string()
    };
    if name.chars().any(char::is_control) {
        return None;
    }
    let actual = classify_folder(&name, &attrs);
    (actual == Some(role)
        || (gmail
            && role == SpecialFolderRole::Archive
            && actual == Some(SpecialFolderRole::AllMail)))
    .then_some(name)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn maps_copyuid_and_provider_special_folders_without_assuming_source_identity() {
        assert_eq!(copy_uid("* OK [COPYUID 17 1 77] copied", 1), Some((17, 77)));
        assert_eq!(copy_uid("OK [COPYUID 17 2 77] copied", 1), None);
        assert_eq!(
            special_folder(
                "* LIST (\\All) \"/\" \"[Gmail]/All Mail\"",
                SpecialFolderRole::Archive,
                true
            ),
            Some("[Gmail]/All Mail".into())
        );
        assert_eq!(
            special_folder(
                "* LIST (\\All) \"/\" \"All Mail\"",
                SpecialFolderRole::Archive,
                false
            ),
            None
        );
        assert_eq!(
            special_folder(
                "* LIST (\\Trash) \"/\" \"Deleted Items\"",
                SpecialFolderRole::Trash,
                false
            ),
            Some("Deleted Items".into())
        );
        assert_eq!(
            special_folder(
                "* LIST (\\Trash) \"/\" {25}",
                SpecialFolderRole::Trash,
                false
            ),
            None
        );
    }
}

#[cfg(test)]
mod wire_tests {
    use super::*;
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, DuplexStream};

    fn server(
        exchanges: Vec<(&'static str, &'static str)>,
    ) -> (ImapSession<DuplexStream>, tokio::task::JoinHandle<()>) {
        let (client, server) = tokio::io::duplex(8192);
        let task = tokio::spawn(async move {
            let (read, mut write) = tokio::io::split(server);
            let mut reader = BufReader::new(read);
            for (expected, response) in exchanges {
                let mut command = String::new();
                reader.read_line(&mut command).await.unwrap();
                assert_eq!(command, expected);
                write.write_all(response.as_bytes()).await.unwrap();
            }
        });
        (ImapSession::new(client), task)
    }

    #[tokio::test]
    async fn flags_are_stored_only_after_the_uid_generation_is_verified() {
        let (mut session, server) = server(vec![
            (
                "A0001 SELECT \"INBOX\"\r\n",
                "* OK [UIDVALIDITY 7]\r\nA0001 OK selected\r\n",
            ),
            (
                "A0002 UID STORE 1 +FLAGS.SILENT (\\Seen)\r\n",
                "A0002 OK stored\r\n",
            ),
        ]);
        let request = ImapMutationRequest {
            folder: "INBOX",
            uid: 1,
            uidvalidity: 7,
            action: ImapMutation::Read(true),
        };
        assert!(matches!(
            execute_imap_mutation(
                &mut session,
                &ServerCapabilities::default(),
                ProviderKind::GenericRfc,
                &request
            )
            .await
            .unwrap(),
            MutationOutcome::FlagsUpdated
        ));
        server.await.unwrap();
        let (mut session, server) = self::server(vec![(
            "A0001 SELECT \"INBOX\"\r\n",
            "* OK [UIDVALIDITY 8]\r\nA0001 OK selected\r\n",
        )]);
        assert!(matches!(
            execute_imap_mutation(
                &mut session,
                &ServerCapabilities::default(),
                ProviderKind::GenericRfc,
                &request
            )
            .await,
            Err(MutationError::StaleIdentity)
        ));
        server.await.unwrap();
    }

    #[tokio::test]
    async fn move_reads_untagged_copyuid_instead_of_reusing_the_source_uid() {
        let (mut session, server) = server(vec![
            (
                "A0001 SELECT \"INBOX\"\r\n",
                "* OK [UIDVALIDITY 7]\r\nA0001 OK selected\r\n",
            ),
            (
                "A0002 LIST \"\" \"*\"\r\n",
                "* LIST (\\Archive) \"/\" \"Archive\"\r\nA0002 OK listed\r\n",
            ),
            (
                "A0003 UID MOVE 1 \"Archive\"\r\n",
                "* OK [COPYUID 17 1 77]\r\n* 1 EXPUNGE\r\nA0003 OK moved\r\n",
            ),
        ]);
        let request = ImapMutationRequest {
            folder: "INBOX",
            uid: 1,
            uidvalidity: 7,
            action: ImapMutation::MoveTo(SpecialFolderRole::Archive),
        };
        let caps = ServerCapabilities::from_tokens(["MOVE"]);
        assert!(matches!(
            execute_imap_mutation(&mut session, &caps, ProviderKind::GenericRfc, &request)
                .await
                .unwrap(),
            MutationOutcome::Moved {
                uid: Some(77),
                uidvalidity: Some(17),
                ..
            }
        ));
        server.await.unwrap();
    }

    #[tokio::test]
    async fn checked_attachment_access_uses_only_the_explicit_peek_section() {
        let (mut session, server) = server(vec![
            (
                "A0001 SELECT \"INBOX\"\r\n",
                "* OK [UIDVALIDITY 7]\r\nA0001 OK selected\r\n",
            ),
            (
                "A0002 UID FETCH 1 (BODY.PEEK[2])\r\n",
                "* 1 FETCH (UID 1 BODY[2] {3}\r\nabc\r\n)\r\nA0002 OK fetched\r\n",
            ),
        ]);
        assert_eq!(
            fetch_attachment_checked(&mut session, "INBOX", 1, "2", 7)
                .await
                .unwrap(),
            b"abc"
        );
        server.await.unwrap();
    }
}
