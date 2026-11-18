//! Native Linux desktop portal. No helper process, web service or file content in IPC.
use crate::service::IpcError;
use std::path::PathBuf;

/// Sanitizes an untrusted sender-supplied attachment filename before passing it to the OS save dialog.
pub fn sanitize_suggested_filename(filename: &str) -> String {
    // INVARIANT: Sender filenames cannot supply paths, controls, or bidi display overrides.
    let basename = filename.rsplit(['/', '\\']).next().unwrap_or("");
    let cleaned: String = basename
        .chars()
        .map(|c| {
            if matches!(
                c,
                '\u{0000}'..='\u{001f}'
                    | '\u{007f}'..='\u{009f}'
                    | '\u{202a}'..='\u{202e}'
                    | '\u{2066}'..='\u{2069}'
                    | '<'
                    | '>'
                    | ':'
                    | '"'
                    | '|'
                    | '?'
                    | '*'
            ) {
                '_'
            } else {
                c
            }
        })
        .collect();
    let trimmed = cleaned
        .trim_matches(|c: char| c == '.' || c.is_whitespace())
        .chars()
        .take(180)
        .collect::<String>();
    if trimmed.is_empty() {
        "attachment".to_string()
    } else {
        trimmed
    }
}

#[cfg(target_os = "linux")]
pub async fn choose(save: bool) -> Result<Option<PathBuf>, IpcError> {
    choose_with_name(save, None).await
}

#[cfg(target_os = "linux")]
pub async fn choose_save_with_name(
    suggested_name: Option<&str>,
) -> Result<Option<PathBuf>, IpcError> {
    choose_with_name(true, suggested_name).await
}

#[cfg(target_os = "linux")]
async fn choose_with_name(
    save: bool,
    suggested_name: Option<&str>,
) -> Result<Option<PathBuf>, IpcError> {
    tokio::time::timeout(
        std::time::Duration::from_secs(300),
        choose_linux(save, suggested_name),
    )
    .await
    .map_err(|_| IpcError::InvalidRequest)?
}

#[cfg(not(target_os = "linux"))]
pub async fn choose(_save: bool) -> Result<Option<PathBuf>, IpcError> {
    Err(IpcError::Unsupported)
}

#[cfg(not(target_os = "linux"))]
pub async fn choose_save_with_name(
    _suggested_name: Option<&str>,
) -> Result<Option<PathBuf>, IpcError> {
    Err(IpcError::Unsupported)
}

#[cfg(target_os = "linux")]
async fn choose_linux(
    save: bool,
    suggested_name: Option<&str>,
) -> Result<Option<PathBuf>, IpcError> {
    use std::{collections::HashMap, pin::Pin};
    use zbus::{
        export::futures_core::Stream,
        zvariant::{OwnedValue, Value},
    };
    let connection = zbus::Connection::session()
        .await
        .map_err(|_| IpcError::Unsupported)?;
    let token = format!(
        "triage_{}",
        vault::backup::random_id().map_err(|_| IpcError::Internal)?
    );
    let sender = connection
        .unique_name()
        .ok_or(IpcError::Internal)?
        .as_str()
        .trim_start_matches(':')
        .replace('.', "_");
    let request_path = format!("/org/freedesktop/portal/desktop/request/{sender}/{token}");
    // INVARIANT: Subscribe before opening the chooser so an immediate response/cancel
    // cannot race signal registration. Only explicit user selection supplies a path.
    let request = zbus::Proxy::new(
        &connection,
        "org.freedesktop.portal.Desktop",
        request_path.as_str(),
        "org.freedesktop.portal.Request",
    )
    .await
    .map_err(|_| IpcError::Unsupported)?;
    let mut responses = request
        .receive_signal("Response")
        .await
        .map_err(|_| IpcError::Unsupported)?;
    let chooser = zbus::Proxy::new(
        &connection,
        "org.freedesktop.portal.Desktop",
        "/org/freedesktop/portal/desktop",
        "org.freedesktop.portal.FileChooser",
    )
    .await
    .map_err(|_| IpcError::Unsupported)?;
    let sanitized_name = if save {
        suggested_name.map(sanitize_suggested_filename)
    } else {
        None
    };
    let mut options = HashMap::from([
        ("handle_token", Value::from(token.as_str())),
        ("modal", Value::from(true)),
    ]);
    if let Some(ref current_name) = sanitized_name {
        options.insert("current_name", Value::from(current_name.as_str()));
    }
    let method = if save { "SaveFile" } else { "OpenFile" };
    let _: zbus::zvariant::OwnedObjectPath = chooser
        .call(method, &("", "Triage Mail", options))
        .await
        .map_err(|_| IpcError::Unsupported)?;
    let response = std::future::poll_fn(|cx| Pin::new(&mut responses).poll_next(cx))
        .await
        .ok_or(IpcError::InvalidRequest)?;
    let (code, results): (u32, HashMap<String, OwnedValue>) = response
        .body()
        .deserialize()
        .map_err(|_| IpcError::InvalidRequest)?;
    if code != 0 {
        return Ok(None);
    }
    let uris: Vec<String> = results
        .get("uris")
        .ok_or(IpcError::InvalidRequest)?
        .try_clone()
        .map_err(|_| IpcError::InvalidRequest)?
        .try_into()
        .map_err(|_| IpcError::InvalidRequest)?;
    if uris.len() != 1 {
        return Err(IpcError::InvalidRequest);
    }
    let url = url::Url::parse(&uris[0]).map_err(|_| IpcError::InvalidRequest)?;
    Ok(Some(
        url.to_file_path().map_err(|_| IpcError::InvalidRequest)?,
    ))
}

#[cfg(test)]
mod tests {
    use super::sanitize_suggested_filename;

    #[test]
    fn sanitizes_untrusted_suggested_attachment_filenames() {
        assert_eq!(
            sanitize_suggested_filename("..\\folder/\u{202e} invoice?.pdf"),
            "_ invoice_.pdf"
        );
        assert_eq!(sanitize_suggested_filename("../../.."), "attachment");
        assert_eq!(sanitize_suggested_filename("   ...  "), "attachment");
        assert_eq!(
            sanitize_suggested_filename("report\x00\x1f<2026>.pdf"),
            "report___2026_.pdf"
        );
        let long = "a".repeat(250);
        assert_eq!(sanitize_suggested_filename(&long).len(), 180);
    }
}
