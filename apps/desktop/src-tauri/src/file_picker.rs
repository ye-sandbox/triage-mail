//! Native Linux desktop portal. No helper process, web service or file content in IPC.
use crate::service::IpcError;
use std::path::PathBuf;

#[cfg(target_os = "linux")]
pub async fn choose(save: bool) -> Result<Option<PathBuf>, IpcError> {
    tokio::time::timeout(std::time::Duration::from_secs(300), choose_linux(save))
        .await
        .map_err(|_| IpcError::InvalidRequest)?
}
#[cfg(not(target_os = "linux"))]
pub async fn choose(_save: bool) -> Result<Option<PathBuf>, IpcError> {
    Err(IpcError::Unsupported)
}

#[cfg(target_os = "linux")]
async fn choose_linux(save: bool) -> Result<Option<PathBuf>, IpcError> {
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
    let options = HashMap::from([
        ("handle_token", Value::from(token.as_str())),
        ("modal", Value::from(true)),
    ]);
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
