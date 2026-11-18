//! Consent, vault staging and dispatch orchestration. Validation never sends test requests.
use crate::{DesktopService, IpcError};
use async_trait::async_trait;
use std::{sync::Arc, time::Duration};
use triage_ai::alerts::{DeliveryAdapter, DeliveryOutcome, Destination, DestinationKind};
use vault::{
    alerts::{IncomingWebhook, WebhookSecret},
    SecretStore, VaultKey,
};
use zeroize::Zeroizing;
mod native;
#[path = "alert_network.rs"]
mod network;

#[async_trait]
pub trait AlertTransport: Send + Sync {
    async fn available(&self) -> bool;
    async fn send(
        &self,
        destination: &Destination,
        secret: Option<&WebhookSecret>,
        payload: &str,
        event: &str,
    ) -> (DeliveryOutcome, i64);
}
pub struct NativeAlerts;
#[async_trait]
impl AlertTransport for NativeAlerts {
    async fn available(&self) -> bool {
        native::available().await
    }
    async fn send(
        &self,
        d: &Destination,
        secret: Option<&WebhookSecret>,
        payload: &str,
        event: &str,
    ) -> (DeliveryOutcome, i64) {
        if d.kind == DestinationKind::Local {
            let (summary, body) = format_local_notification(d, payload);
            return (native::notify(&summary, &body).await, 0);
        }
        let Some(secret) = secret else {
            return (DeliveryOutcome::Unavailable, 0);
        };
        if matches!(d.kind, DestinationKind::Telegram | DestinationKind::Discord) {
            return send_connector(d.kind, secret, payload, event, &SystemResolver).await;
        }
        let adapter = WebhookAdapter { secret };
        // DeliveryAdapter is usable by headless compositions without a desktop service.
        adapter.deliver(payload, event).await
    }
}
struct WebhookAdapter<'a> {
    secret: &'a WebhookSecret,
}
impl DeliveryAdapter for WebhookAdapter<'_> {
    fn deliver<'a>(
        &'a self,
        payload: &'a str,
        event: &'a str,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = (DeliveryOutcome, i64)> + Send + 'a>>
    {
        Box::pin(async move { send_webhook(self.secret, payload, event).await })
    }
}
fn storage_error(_: storage::StorageError) -> IpcError {
    IpcError::StorageUnavailable
}
fn key(id: &str) -> Result<VaultKey, IpcError> {
    VaultKey::for_account("alerts", id).map_err(|_| IpcError::Internal)
}
fn read_secret(vault: &dyn SecretStore, id: &str) -> Result<WebhookSecret, IpcError> {
    let raw = Zeroizing::new(
        vault
            .get_secret(&key(id)?)
            .map_err(|_| IpcError::VaultUnavailable)?,
    );
    serde_json::from_slice(&raw).map_err(|_| IpcError::VaultUnavailable)
}
pub(crate) fn clean_secrets(service: &DesktopService) -> Result<(), IpcError> {
    for id in service
        .store
        .orphan_alert_secrets()
        .map_err(storage_error)?
    {
        match service.vault.delete_secret(&key(&id)?) {
            Ok(()) | Err(vault::VaultError::NotFound(_)) => service
                .store
                .forget_alert_secret(&id)
                .map_err(storage_error)?,
            Err(_) => return Err(IpcError::VaultUnavailable),
        }
    }
    Ok(())
}
/// Syntax validation only. DNS is deferred until an enabled job dispatches.
fn webhook_url(secret: &WebhookSecret) -> Result<url::Url, IpcError> {
    if secret.url.len() > 4096
        || secret
            .bearer
            .as_ref()
            .is_some_and(|s| s.len() > 4096 || s.chars().any(char::is_control))
    {
        return Err(IpcError::InvalidRequest);
    }
    let url = url::Url::parse(&secret.url).map_err(|_| IpcError::InvalidRequest)?;
    let host = url.host_str().ok_or(IpcError::InvalidRequest)?;
    if url.scheme() != "https"
        || url.port_or_known_default() != Some(443)
        || !url.username().is_empty()
        || url.password().is_some()
        || url.fragment().is_some()
        || !matches!(url.host(), Some(url::Host::Domain(_)))
        || !host.contains('.')
        || host.ends_with('.')
        || host.ends_with(".localhost")
    {
        return Err(IpcError::InvalidRequest);
    }
    Ok(url)
}
/// INVARIANT: Provider hosts and paths are fixed; no setup request or URL-derived discovery.
fn connector_url(kind: DestinationKind, secret: &WebhookSecret) -> Result<url::Url, IpcError> {
    let url = webhook_url(secret)?;
    let valid = match kind {
        DestinationKind::Webhook => true,
        DestinationKind::Telegram => {
            url.host_str() == Some("api.telegram.org")
                && url.query().is_none()
                && url
                    .path()
                    .strip_prefix("/bot")
                    .and_then(|p| p.strip_suffix("/sendMessage"))
                    .is_some_and(vault::alerts::valid_bot_token)
                && secret
                    .bearer
                    .as_deref()
                    .is_some_and(vault::alerts::valid_chat_id)
        }
        DestinationKind::Discord => {
            let parts: Vec<_> = url.path().split('/').collect();
            url.host_str() == Some("discord.com")
                && url.query().is_none()
                && secret.bearer.is_none()
                && parts.len() == 5
                && parts[1] == "api"
                && parts[2] == "webhooks"
                && !parts[3].is_empty()
                && parts[3].len() <= 20
                && parts[3].bytes().all(|c| c.is_ascii_digit())
                && !parts[4].is_empty()
                && parts[4].len() <= 256
                && parts[4]
                    .bytes()
                    .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'_' | b'-'))
        }
        DestinationKind::Local => false,
    };
    if valid {
        Ok(url)
    } else {
        Err(IpcError::InvalidRequest)
    }
}

impl DesktopService {
    pub async fn list_alerts(self: &Arc<Self>) -> Result<Vec<Destination>, IpcError> {
        self.blocking(|s| s.store.list_alert_destinations().map_err(storage_error))
            .await
    }
    pub async fn local_alert_available(self: &Arc<Self>) -> bool {
        tokio::time::timeout(Duration::from_secs(3), self.alert_adapter.available())
            .await
            .unwrap_or(false)
    }
    pub async fn save_alert(
        self: &Arc<Self>,
        mut d: Destination,
        secret: Option<IncomingWebhook>,
        consent: bool,
    ) -> Result<Destination, IpcError> {
        let _gate = self.alert_gate.lock().await;
        if !d.validate() || (d.enabled && !consent) {
            return Err(IpcError::InvalidRequest);
        }
        if d.kind == DestinationKind::Local && d.enabled && !self.local_alert_available().await {
            return Err(IpcError::Unsupported);
        }
        self.blocking(move |s| {
            clean_secrets(&s)?;
            let current = s
                .store
                .list_alert_destinations()
                .map_err(storage_error)?
                .into_iter()
                .find(|v| v.id == d.id);
            if current.as_ref().map_or(0, |v| v.revision) != d.revision {
                return Err(IpcError::InvalidRequest);
            }
            let old_key = if current.is_some() {
                s.store.alert_secret_key(&d.id).map_err(storage_error)?
            } else {
                None
            };
            let mut staged = None;
            if d.kind != DestinationKind::Local {
                let material = match secret {
                    Some(value) => value.0,
                    None => {
                        if current.as_ref().is_none_or(|v| v.kind != d.kind) {
                            return Err(IpcError::InvalidRequest);
                        }
                        read_secret(
                            s.vault.as_ref(),
                            old_key.as_deref().ok_or(IpcError::InvalidRequest)?,
                        )?
                    }
                };
                let url = connector_url(d.kind, &material)?;
                let origin = Some(url.origin().ascii_serialization());
                // INVARIANT: Consent binds to the public origin actually displayed by the UI.
                if d.enabled && d.public_origin != origin {
                    return Err(IpcError::InvalidRequest);
                }
                d.public_origin = origin;
                d.has_credentials =
                    matches!(d.kind, DestinationKind::Telegram | DestinationKind::Discord)
                        || material.bearer.as_ref().is_some_and(|v| !v.is_empty());
                let id = crate::service::opaque_id()?;
                s.store.register_alert_secret(&id).map_err(storage_error)?;
                let raw = Zeroizing::new(
                    serde_json::to_vec(&material).map_err(|_| IpcError::InvalidRequest)?,
                );
                s.vault
                    .set_secret(&key(&id)?, &raw)
                    .map_err(|_| IpcError::VaultUnavailable)?;
                staged = Some(id);
            } else {
                d.public_origin = None;
                d.has_credentials = false;
            }
            let result = s
                .store
                .save_alert_destination(d, staged.as_deref(), storage::now_epoch_ms())
                .map_err(storage_error);
            let _ = clean_secrets(&s);
            result
        })
        .await
    }
    pub async fn delete_alert(self: &Arc<Self>, id: String, revision: u32) -> Result<(), IpcError> {
        let _gate = self.alert_gate.lock().await;
        self.blocking(move |s| {
            s.store
                .delete_alert_destination(&id, revision)
                .map_err(storage_error)?;
            clean_secrets(&s)
        })
        .await
    }
    pub async fn alert_history(
        self: &Arc<Self>,
    ) -> Result<Vec<storage::alerts::DeliveryHistory>, IpcError> {
        self.blocking(|s| s.store.alert_history().map_err(storage_error))
            .await
    }
    pub async fn test_alert(
        self: &Arc<Self>,
        id: String,
        revision: u32,
        consent: bool,
    ) -> Result<(), IpcError> {
        if !consent {
            return Err(IpcError::InvalidRequest);
        }
        let _gate = self.alert_gate.lock().await;
        self.blocking(move |s| {
            if !s
                .store
                .list_alert_destinations()
                .map_err(storage_error)?
                .iter()
                .any(|d| d.id == id && d.revision == revision && d.enabled)
            {
                return Err(IpcError::InvalidRequest);
            }
            s.store
                .enqueue_alert_test(
                    &id,
                    &format!("test-{}", crate::service::opaque_id()?),
                    storage::now_epoch_ms(),
                )
                .map_err(storage_error)
        })
        .await
    }
    pub async fn replay_alert(
        self: &Arc<Self>,
        ids: Vec<String>,
        destination: String,
        revision: u32,
        consent: bool,
    ) -> Result<(), IpcError> {
        if !consent {
            return Err(IpcError::InvalidRequest);
        }
        let _gate = self.alert_gate.lock().await;
        self.blocking(move |s| {
            if !s
                .store
                .list_alert_destinations()
                .map_err(storage_error)?
                .iter()
                .any(|d| d.id == destination && d.revision == revision && d.enabled)
            {
                return Err(IpcError::InvalidRequest);
            }
            s.store
                .replay_alerts(
                    &ids,
                    &destination,
                    &format!("replay-{}", crate::service::opaque_id()?),
                    storage::now_epoch_ms(),
                )
                .map_err(storage_error)
        })
        .await
    }
    pub async fn process_alerts(self: &Arc<Self>) -> Result<(), IpcError> {
        // INVARIANT: Disable/delete cannot race a new dispatch; no SQLite lock crosses I/O.
        // One bounded attempt per worker tick also bounds dispatch across destinations.
        let _gate = self.alert_gate.lock().await;
        let request = self
            .blocking(|s| {
                clean_secrets(&s)?;
                s.store
                    .process_alert_schedules(
                        storage::now_epoch_ms(),
                        &crate::alert_schedule::NativeCalendar,
                    )
                    .map_err(storage_error)?;
                let job = s
                    .store
                    .claim_alert_with_calendar(
                        storage::now_epoch_ms(),
                        &crate::alert_schedule::NativeCalendar,
                    )
                    .map_err(storage_error)?;
                Ok(job)
            })
            .await?;
        if let Some(job) = request {
            let secret = if let Some(id) = job.secret_key.clone() {
                self.blocking(move |s| read_secret(s.vault.as_ref(), &id))
                    .await
                    .ok()
            } else {
                None
            };
            let outcome = tokio::time::timeout(
                Duration::from_secs(10),
                self.alert_adapter
                    .send(&job.destination, secret.as_ref(), &job.payload, &job.id),
            )
            .await
            .unwrap_or((DeliveryOutcome::Retry, 0));
            self.blocking(move |s| {
                let mut bytes = [0; 2];
                getrandom::getrandom(&mut bytes).map_err(|_| IpcError::Internal)?;
                s.store
                    .finish_alert(
                        &job,
                        outcome.0,
                        outcome.1,
                        i64::from(u16::from_le_bytes(bytes)) % 5001,
                        storage::now_epoch_ms(),
                    )
                    .map_err(storage_error)
            })
            .await?;
        }
        Ok(())
    }
}
#[async_trait]
trait WebhookResolver: Send + Sync {
    async fn resolve(&self, host: &str) -> Result<Vec<std::net::SocketAddr>, ()>;
}
struct SystemResolver;
#[async_trait]
impl WebhookResolver for SystemResolver {
    async fn resolve(&self, host: &str) -> Result<Vec<std::net::SocketAddr>, ()> {
        tokio::net::lookup_host((host, 443))
            .await
            .map(|a| a.collect())
            .map_err(|_| ())
    }
}
async fn send_webhook(
    secret: &WebhookSecret,
    payload: &str,
    event: &str,
) -> (DeliveryOutcome, i64) {
    send_with_resolver(secret, payload, event, &SystemResolver).await
}
async fn send_with_resolver(
    secret: &WebhookSecret,
    payload: &str,
    event: &str,
    resolver: &dyn WebhookResolver,
) -> (DeliveryOutcome, i64) {
    let Ok(url) = webhook_url(secret) else {
        return (DeliveryOutcome::Rejected, 0);
    };
    let Some(host) = url.host_str() else {
        return (DeliveryOutcome::Rejected, 0);
    };
    let client = match public_client(host, resolver).await {
        Ok(client) => client,
        Err(outcome) => return (outcome, 0),
    };
    let mut request = client
        .post(url)
        .header("Content-Type", "application/json")
        .header("Idempotency-Key", event)
        .body(payload.to_string());
    if let Some(bearer) = &secret.bearer {
        request = request.bearer_auth(bearer);
    }
    let Ok(response) = request.send().await else {
        return (DeliveryOutcome::Retry, 0);
    };
    let status = response.status().as_u16();
    // INVARIANT: Never read receiver content. Header size is bounded by the HTTP implementation.
    let retry = response
        .headers()
        .get("retry-after")
        .and_then(|h| h.to_str().ok())
        .map(retry_after)
        .unwrap_or(0);
    (classify_status(status), retry)
}
async fn public_client(
    host: &str,
    resolver: &dyn WebhookResolver,
) -> Result<reqwest::Client, DeliveryOutcome> {
    let addresses = tokio::time::timeout(Duration::from_secs(3), resolver.resolve(host))
        .await
        .map_err(|_| DeliveryOutcome::Retry)?
        .map_err(|_| DeliveryOutcome::Retry)?;
    // INVARIANT: Pin the validated public snapshot; never resolve again inside the connector.
    if addresses.is_empty()
        || addresses.len() > 32
        || addresses
            .iter()
            .any(|a| a.port() != 443 || !network::public_address(a.ip()))
    {
        return Err(DeliveryOutcome::Rejected);
    }
    reqwest::Client::builder()
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .min_tls_version(reqwest::tls::Version::TLS_1_2)
        .resolve_to_addrs(host, &addresses)
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|_| DeliveryOutcome::Unavailable)
}
async fn send_connector(
    kind: DestinationKind,
    secret: &WebhookSecret,
    payload: &str,
    event: &str,
    resolver: &dyn WebhookResolver,
) -> (DeliveryOutcome, i64) {
    tokio::time::timeout(
        Duration::from_secs(10),
        connector_request(kind, secret, payload, event, resolver),
    )
    .await
    .unwrap_or((DeliveryOutcome::Retry, 0))
}
async fn connector_request(
    kind: DestinationKind,
    secret: &WebhookSecret,
    payload: &str,
    event: &str,
    resolver: &dyn WebhookResolver,
) -> (DeliveryOutcome, i64) {
    use triage_ai::alerts::connectors;
    let Ok(url) = connector_url(kind, secret) else {
        return (DeliveryOutcome::Rejected, 0);
    };
    let Some(body) = connectors::message(kind, payload, secret.bearer.as_deref()) else {
        return (DeliveryOutcome::Rejected, 0);
    };
    let Some(host) = url.host_str() else {
        return (DeliveryOutcome::Rejected, 0);
    };
    let client = match public_client(host, resolver).await {
        Ok(client) => client,
        Err(outcome) => return (outcome, 0),
    };
    // INVARIANT: Telegram's vault bearer slot holds the chat ID, never HTTP authorization.
    // Providers do not promise Idempotency-Key support; event identity remains in the text.
    let Ok(mut response) = client
        .post(url)
        .header("Content-Type", "application/json")
        .header("Idempotency-Key", event)
        .body(body)
        .send()
        .await
    else {
        return (DeliveryOutcome::Retry, 0);
    };
    let status = response.status().as_u16();
    let retry = response
        .headers()
        .get("retry-after")
        .and_then(|h| h.to_str().ok())
        .map(retry_after)
        .unwrap_or(0);
    let mut bytes = Zeroizing::new(Vec::new());
    loop {
        match response.chunk().await {
            Ok(Some(chunk)) if bytes.len() + chunk.len() <= connectors::MAX_RESPONSE_BYTES => {
                bytes.extend_from_slice(&chunk)
            }
            Ok(Some(_)) => return (DeliveryOutcome::Rejected, 0),
            Ok(None) => break,
            Err(_) => return (DeliveryOutcome::Retry, retry),
        }
    }
    let (outcome, provider_retry) = connectors::response(kind, status, &bytes);
    (
        outcome,
        if outcome == DeliveryOutcome::Retry {
            retry.max(provider_retry)
        } else {
            0
        },
    )
}
fn classify_status(status: u16) -> DeliveryOutcome {
    match status {
        200..=299 => DeliveryOutcome::Accepted,
        408 | 429 | 500..=599 => DeliveryOutcome::Retry,
        _ => DeliveryOutcome::Rejected,
    }
}
fn retry_after(value: &str) -> i64 {
    if let Ok(seconds) = value.parse::<u64>() {
        return seconds.min(3600) as i64 * 1000;
    }
    chrono::DateTime::parse_from_rfc2822(value)
        .ok()
        .map(|date| (date.timestamp_millis() - storage::now_epoch_ms()).clamp(0, 3_600_000))
        .unwrap_or(0)
}

const DEFAULT_LOCAL_SUMMARY: &str = "New mail";
const DEFAULT_LOCAL_BODY: &str = "A new message matched an alert rule.";
const MAX_SUMMARY_CHARS: usize = 100;
const MAX_BODY_CHARS: usize = 240;

fn scrub_text(input: &str, max_chars: usize) -> String {
    if max_chars == 0 {
        return String::new();
    }
    let mut out = String::new();
    let mut prev_space = true;
    let mut count = 0usize;
    for ch in input.chars() {
        let normalized = if matches!(ch, '\n' | '\r' | '\t' | ' ') {
            Some(' ')
        } else if ch.is_control() {
            None
        } else {
            Some(ch)
        };
        let Some(c) = normalized else {
            continue;
        };
        if c == ' ' {
            if prev_space {
                continue;
            }
            prev_space = true;
        } else {
            prev_space = false;
        }
        if count >= max_chars {
            if out.ends_with(' ') {
                out.pop();
            }
            out.push('…');
            return out;
        }
        out.push(c);
        count += 1;
    }
    if out.ends_with(' ') {
        out.pop();
    }
    out
}

fn format_extracted_facts(facts: &serde_json::Value) -> Option<String> {
    let mut parts = Vec::new();
    if let Some(due_dates) = facts.get("due_dates").and_then(serde_json::Value::as_array) {
        if let Some(first_due) = due_dates
            .iter()
            .filter_map(serde_json::Value::as_str)
            .map(|d| scrub_text(d, 16))
            .find(|d| !d.is_empty())
        {
            parts.push(format!("Due {first_due}"));
        }
    }
    if let Some(amounts) = facts
        .get("amounts_cents")
        .and_then(serde_json::Value::as_array)
    {
        if let Some(cents) = amounts.iter().find_map(serde_json::Value::as_u64) {
            parts.push(format!("Amount {}.{:02}", cents / 100, cents % 100));
        }
    }
    (!parts.is_empty()).then(|| parts.join(" · "))
}

fn format_attachments(attachments: &serde_json::Value) -> Option<String> {
    let list = attachments.as_array()?;
    if list.is_empty() {
        return None;
    }
    let first_name = list
        .first()
        .and_then(|item| item.get("name"))
        .and_then(serde_json::Value::as_str)
        .map(|name| scrub_text(name, 48))
        .filter(|name| !name.is_empty());
    match (list.len(), first_name) {
        (1, Some(name)) => Some(format!("Attachment: {name}")),
        (n, Some(name)) => Some(format!("{n} attachments ({name})")),
        (1, None) => Some("1 attachment".into()),
        (n, None) => Some(format!("{n} attachments")),
    }
}

/// INVARIANT: Formats title and body strictly from the already field-consented payload JSON.
/// Never queries storage or enriches beyond consented fields; zero shell or remote asset syntax.
pub(crate) fn format_local_notification(
    destination: &Destination,
    payload: &str,
) -> (String, String) {
    if payload.len() > triage_ai::alerts::MAX_PAYLOAD_BYTES {
        return (DEFAULT_LOCAL_SUMMARY.into(), DEFAULT_LOCAL_BODY.into());
    }
    let Ok(serde_json::Value::Object(obj)) = serde_json::from_str::<serde_json::Value>(payload)
    else {
        return (DEFAULT_LOCAL_SUMMARY.into(), DEFAULT_LOCAL_BODY.into());
    };

    let dest_name = scrub_text(&destination.name, 40);

    if obj.get("test").and_then(serde_json::Value::as_bool) == Some(true) {
        let summary = if dest_name.is_empty() {
            "triage-mail · Test alert".into()
        } else {
            scrub_text(&format!("triage-mail · {dest_name}"), MAX_SUMMARY_CHARS)
        };
        return (summary, "Local desktop notification test succeeded.".into());
    }

    if obj.get("digest").and_then(serde_json::Value::as_bool) == Some(true) {
        let total = obj
            .get("total")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(0);
        let summary = if dest_name.is_empty() {
            scrub_text(&format!("Mail digest ({total})"), MAX_SUMMARY_CHARS)
        } else {
            scrub_text(
                &format!("{dest_name} · Digest ({total})"),
                MAX_SUMMARY_CHARS,
            )
        };

        let mut previews = Vec::new();
        if let Some(entries) = obj.get("entries").and_then(serde_json::Value::as_array) {
            for entry in entries.iter().take(3) {
                let subj = entry
                    .get("subject")
                    .and_then(serde_json::Value::as_str)
                    .map(|s| scrub_text(s, 48))
                    .filter(|s| !s.is_empty());
                let sender = entry
                    .get("sender")
                    .and_then(serde_json::Value::as_str)
                    .map(|s| scrub_text(s, 40))
                    .filter(|s| !s.is_empty());
                let facts = entry
                    .get("extracted_facts")
                    .and_then(format_extracted_facts);
                let item = match (subj, sender, facts) {
                    (Some(su), Some(se), _) => format!("{su} ({se})"),
                    (Some(su), None, Some(fa)) => format!("{su} [{fa}]"),
                    (Some(su), None, None) => su,
                    (None, Some(se), Some(fa)) => format!("{se} [{fa}]"),
                    (None, Some(se), None) => se,
                    (None, None, Some(fa)) => fa,
                    (None, None, None) => continue,
                };
                previews.push(item);
            }
        }
        let body = if previews.is_empty() {
            scrub_text(
                &format!("{total} matching message(s) queued in this digest."),
                MAX_BODY_CHARS,
            )
        } else {
            scrub_text(
                &format!("{total} alerts: {}", previews.join(" · ")),
                MAX_BODY_CHARS,
            )
        };
        return (summary, body);
    }

    let sender = obj
        .get("sender")
        .and_then(serde_json::Value::as_str)
        .map(|s| scrub_text(s, 80))
        .filter(|s| !s.is_empty());
    let subject = obj
        .get("subject")
        .and_then(serde_json::Value::as_str)
        .map(|s| scrub_text(s, 96))
        .filter(|s| !s.is_empty());
    let preview = obj
        .get("excerpt")
        .or_else(|| obj.get("body"))
        .and_then(serde_json::Value::as_str)
        .map(|s| scrub_text(s, 140))
        .filter(|s| !s.is_empty());
    let facts = obj.get("extracted_facts").and_then(format_extracted_facts);
    let attachments = obj.get("attachments").and_then(format_attachments);
    let account = obj
        .get("account_id")
        .and_then(serde_json::Value::as_str)
        .map(|s| scrub_text(s, 48))
        .filter(|s| !s.is_empty());

    let has_consented_detail = sender.is_some()
        || subject.is_some()
        || preview.is_some()
        || facts.is_some()
        || attachments.is_some()
        || account.is_some();

    if !has_consented_detail {
        return (DEFAULT_LOCAL_SUMMARY.into(), DEFAULT_LOCAL_BODY.into());
    }

    let summary = match (&dest_name, &subject, &sender) {
        (d, Some(subj), _) if !d.is_empty() => {
            scrub_text(&format!("{d}: {subj}"), MAX_SUMMARY_CHARS)
        }
        (_, Some(subj), _) => scrub_text(subj, MAX_SUMMARY_CHARS),
        (d, None, Some(snd)) if !d.is_empty() => {
            scrub_text(&format!("{d}: {snd}"), MAX_SUMMARY_CHARS)
        }
        (_, None, Some(snd)) => scrub_text(snd, MAX_SUMMARY_CHARS),
        (d, None, None) if !d.is_empty() => {
            scrub_text(&format!("{d}: {DEFAULT_LOCAL_SUMMARY}"), MAX_SUMMARY_CHARS)
        }
        _ => DEFAULT_LOCAL_SUMMARY.into(),
    };

    let mut body_parts = Vec::new();
    if let Some(snd) = sender {
        body_parts.push(format!("From: {snd}"));
    }
    if let Some(prev) = preview {
        body_parts.push(prev);
    }
    if let Some(f) = facts {
        body_parts.push(f);
    }
    if let Some(att) = attachments {
        body_parts.push(att);
    }
    if let Some(acct) = account {
        body_parts.push(format!("Account: {acct}"));
    }

    let body = if body_parts.is_empty() {
        DEFAULT_LOCAL_BODY.into()
    } else {
        scrub_text(&body_parts.join(" · "), MAX_BODY_CHARS)
    };

    (summary, body)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_unsafe_urls_and_keeps_diagnostics_secret_free() {
        for url in [
            "http://example.com/hook",
            "https://example.com:444/hook",
            "https://127.0.0.1/",
            "https://[::1]/",
            "https://localhost/",
            "https://user:password@example.com/hook",
            "https://example.com/#secret",
        ] {
            let s = WebhookSecret {
                url: url.into(),
                bearer: None,
            };
            assert_eq!(webhook_url(&s), Err(IpcError::InvalidRequest));
            assert!(!format!("{s:?}").contains(url));
        }
        let s = WebhookSecret {
            url: "https://hooks.example.com/private-path?value=synthetic".into(),
            bearer: None,
        };
        assert_eq!(
            webhook_url(&s).unwrap().origin().ascii_serialization(),
            "https://hooks.example.com"
        );
        let err =
            serde_json::from_str::<IncomingWebhook>(r#"{"url":9,"bearer":"synthetic-sensitive"}"#)
                .err()
                .unwrap()
                .to_string();
        assert!(!err.contains("synthetic-sensitive"));
        for ip in [
            "127.0.0.1",
            "10.0.0.1",
            "169.254.169.254",
            "::1",
            "::ffff:8.8.8.8",
            "2001:db8::1",
        ] {
            assert!(!network::public_address(ip.parse().unwrap()));
        }
    }
    #[tokio::test]
    async fn injected_dns_rejects_mixed_empty_and_private_snapshots_before_connecting() {
        struct Resolver(Vec<std::net::SocketAddr>);
        #[async_trait]
        impl WebhookResolver for Resolver {
            async fn resolve(&self, _: &str) -> Result<Vec<std::net::SocketAddr>, ()> {
                Ok(self.0.clone())
            }
        }
        let secret = WebhookSecret {
            url: "https://hooks.example.com/path".into(),
            bearer: None,
        };
        for addresses in [
            vec![],
            vec!["127.0.0.1:443".parse().unwrap()],
            vec![
                "8.8.8.8:443".parse().unwrap(),
                "10.0.0.1:443".parse().unwrap(),
            ],
        ] {
            assert_eq!(
                send_with_resolver(&secret, "{}", "e", &Resolver(addresses))
                    .await
                    .0,
                DeliveryOutcome::Rejected
            );
        }
    }
    #[test]
    fn provider_paths_and_recipient_material_fail_closed() {
        for (kind, url, bearer) in [
            (
                DestinationKind::Telegram,
                "https://api.telegram.org/bot123:synthetic/getUpdates",
                Some("-123"),
            ),
            (
                DestinationKind::Telegram,
                "https://api.telegram.org/bot123:synthetic/sendMessage?redirect=1",
                Some("-123"),
            ),
            (
                DestinationKind::Telegram,
                "https://api.telegram.org.attacker.example/bot123:synthetic/sendMessage",
                Some("-123"),
            ),
            (
                DestinationKind::Telegram,
                "https://api.telegram.org/bot123:synthetic/sendMessage",
                Some("@recipient"),
            ),
            (
                DestinationKind::Discord,
                "https://discord.com/api/webhooks/123/synthetic?thread_id=1",
                None,
            ),
            (
                DestinationKind::Discord,
                "https://discord.com/api/webhooks/123/synthetic/extra",
                None,
            ),
            (
                DestinationKind::Discord,
                "https://discord.com/api/webhooks/123/synthetic",
                Some("unexpected"),
            ),
            (
                DestinationKind::Discord,
                "https://discord.com/api/webhooks/123/synthetic%2Fescape",
                None,
            ),
            (
                DestinationKind::Discord,
                "https://hooks.example.com/api/webhooks/123/synthetic",
                None,
            ),
        ] {
            let secret = WebhookSecret {
                url: url.into(),
                bearer: bearer.map(str::to_string),
            };
            assert!(connector_url(kind, &secret).is_err());
        }
        for (kind, url, bearer) in [
            (
                DestinationKind::Telegram,
                "https://api.telegram.org/bot123:synthetic/sendMessage",
                Some("-123"),
            ),
            (
                DestinationKind::Discord,
                "https://discord.com/api/webhooks/123/synthetic",
                None,
            ),
        ] {
            let secret = WebhookSecret {
                url: url.into(),
                bearer: bearer.map(str::to_string),
            };
            assert!(connector_url(kind, &secret).is_ok());
        }
    }
    #[tokio::test]
    async fn provider_dispatch_rejects_dns_without_a_connection() {
        struct Private;
        #[async_trait]
        impl WebhookResolver for Private {
            async fn resolve(&self, _: &str) -> Result<Vec<std::net::SocketAddr>, ()> {
                Ok(vec!["127.0.0.1:443".parse().unwrap()])
            }
        }
        let secret = WebhookSecret {
            url: "https://discord.com/api/webhooks/123/synthetic".into(),
            bearer: None,
        };
        assert_eq!(
            send_connector(DestinationKind::Discord, &secret, "{}", "e", &Private)
                .await
                .0,
            DeliveryOutcome::Rejected
        );
    }
    #[test]
    fn classification_and_retry_after_are_bounded() {
        assert_eq!(classify_status(202), DeliveryOutcome::Accepted);
        for status in [408, 429, 500, 503] {
            assert_eq!(classify_status(status), DeliveryOutcome::Retry);
        }
        for status in [301, 302, 400, 401, 403, 404] {
            assert_eq!(classify_status(status), DeliveryOutcome::Rejected);
        }
        assert_eq!(retry_after("120"), 120_000);
        assert_eq!(retry_after("999999"), 3_600_000);
        assert_eq!(retry_after("invalid"), 0);
        assert_eq!(retry_after("Fri, 01 Jan 2100 00:00:00 GMT"), 3_600_000);
    }

    #[test]
    fn local_notification_respects_consent_scrubs_controls_and_bounds_lengths() {
        let local_dest = Destination {
            id: "local-1".into(),
            revision: 1,
            name: "Boletos\nUrgentes\u{0000}\u{001b}[31m".into(),
            kind: DestinationKind::Local,
            enabled: true,
            fields: vec![],
            public_origin: None,
            has_credentials: false,
        };

        // Zero consented fields -> redacted generic fallback
        let unconsented = serde_json::json!({
            "schema_version": 1,
            "event_id": "evt-1",
            "timestamp_epoch_ms": 1000,
            "truncated": false
        })
        .to_string();
        assert_eq!(
            format_local_notification(&local_dest, &unconsented),
            (
                "New mail".into(),
                "A new message matched an alert rule.".into()
            )
        );

        // Consented fields -> formatted, control-char-scrubbed, bounded
        let consented = serde_json::json!({
            "schema_version": 1,
            "event_id": "evt-2",
            "timestamp_epoch_ms": 1000,
            "sender": "billing@example.com\r\n\u{0007}",
            "subject": "Conta de Energia\tVencimento\u{001b}",
            "excerpt": "Pagamento via Pix ou boleto.\nLinha digitavel",
            "extracted_facts": {
                "due_dates": ["2026-11-20"],
                "amounts_cents": [14950]
            },
            "attachments": [
                {"name": "fatura.pdf", "mime_type": "application/pdf"}
            ],
            "truncated": false
        })
        .to_string();
        let (summary, body) = format_local_notification(&local_dest, &consented);
        assert!(summary.contains("Boletos Urgentes[31m: Conta de Energia Vencimento"));
        assert!(!summary.chars().any(char::is_control));
        assert!(!body.chars().any(char::is_control));
        assert!(body.contains("From: billing@example.com"));
        assert!(body.contains("Due 2026-11-20"));
        assert!(body.contains("Amount 149.50"));
        assert!(body.contains("Attachment: fatura.pdf"));

        // Oversized unicode input stays bounded with ellipsis
        let huge = serde_json::json!({
            "schema_version": 1,
            "event_id": "evt-3",
            "timestamp_epoch_ms": 1000,
            "subject": "😀".repeat(400),
            "body": "Texto longo ".repeat(100),
            "truncated": true
        })
        .to_string();
        let (huge_summary, huge_body) = format_local_notification(&local_dest, &huge);
        assert!(huge_summary.chars().count() <= MAX_SUMMARY_CHARS + 1);
        assert!(huge_summary.ends_with('…'));
        assert!(huge_body.chars().count() <= MAX_BODY_CHARS + 1);
        assert!(huge_body.ends_with('…'));

        // Digest payload formats summary and respects unconsented vs consented entries
        let digest_unconsented = serde_json::json!({
            "schema_version": 1,
            "event_id": "dig-1",
            "timestamp_epoch_ms": 1000,
            "digest": true,
            "total": 4,
            "omitted": 0,
            "truncated": false,
            "entries": [{"schema_version": 1, "event_id": "e1"}]
        })
        .to_string();
        let (d_sum, d_body) = format_local_notification(&local_dest, &digest_unconsented);
        assert!(d_sum.contains("Digest (4)"));
        assert_eq!(d_body, "4 matching message(s) queued in this digest.");

        let digest_consented = serde_json::json!({
            "schema_version": 1,
            "event_id": "dig-2",
            "timestamp_epoch_ms": 1000,
            "digest": true,
            "total": 2,
            "omitted": 0,
            "truncated": false,
            "entries": [
                {"subject": "Invoice #1", "sender": "a@example.com"},
                {"subject": "Invoice #2", "extracted_facts": {"due_dates": ["2026-11-22"], "amounts_cents": []}}
            ]
        })
        .to_string();
        let (_, dc_body) = format_local_notification(&local_dest, &digest_consented);
        assert!(
            dc_body.contains("2 alerts: Invoice #1 (a@example.com) · Invoice #2 [Due 2026-11-22]")
        );

        // Test payload
        let test_payload = serde_json::json!({
            "schema_version": 1,
            "event_id": "test-1",
            "timestamp_epoch_ms": 1000,
            "test": true
        })
        .to_string();
        let (t_sum, t_body) = format_local_notification(&local_dest, &test_payload);
        assert!(t_sum.starts_with("triage-mail · "));
        assert_eq!(t_body, "Local desktop notification test succeeded.");

        // Malformed payload falls back safely
        assert_eq!(
            format_local_notification(&local_dest, "not-json"),
            (
                "New mail".into(),
                "A new message matched an alert rule.".into()
            )
        );
    }
}

#[cfg(test)]
mod service_tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    struct Fake {
        calls: AtomicUsize,
    }
    #[async_trait]
    impl AlertTransport for Fake {
        async fn available(&self) -> bool {
            true
        }
        async fn send(
            &self,
            _: &Destination,
            secret: Option<&WebhookSecret>,
            payload: &str,
            event: &str,
        ) -> (DeliveryOutcome, i64) {
            assert!(secret.is_some());
            assert!(payload.contains(event));
            self.calls.fetch_add(1, Ordering::SeqCst);
            (DeliveryOutcome::Accepted, 0)
        }
    }
    fn fixture() -> (Arc<DesktopService>, Arc<Fake>) {
        let store = Arc::new(
            storage::SqliteMailStore::open_in_memory(
                &storage::DatabaseKey::from_bytes([34; 32]),
                Default::default(),
            )
            .unwrap(),
        );
        let vault: Arc<dyn SecretStore> = Arc::new(vault::EncryptedFallbackStore::in_memory(
            Arc::new(vault::Argon2AesGcmCipher::from_key([35; 32])),
        ));
        let mut service = DesktopService::new(
            store,
            vault.clone(),
            Arc::new(crate::remote::NativeRemoteMail::new(vault)),
            None,
        );
        let fake = Arc::new(Fake {
            calls: AtomicUsize::new(0),
        });
        Arc::get_mut(&mut service).unwrap().alert_adapter = fake.clone();
        (service, fake)
    }
    fn destination() -> Destination {
        Destination {
            id: "d".into(),
            revision: 0,
            name: "Synthetic".into(),
            kind: DestinationKind::Webhook,
            enabled: true,
            fields: vec![],
            public_origin: Some("https://hooks.example.com".into()),
            has_credentials: false,
        }
    }
    fn secret() -> IncomingWebhook {
        IncomingWebhook(WebhookSecret {
            url: "https://hooks.example.com/private-path".into(),
            bearer: Some("synthetic-sensitive".into()),
        })
    }
    #[tokio::test]
    async fn consent_revision_vault_redaction_and_offline_dispatch() {
        let (s, fake) = fixture();
        assert_eq!(
            s.save_alert(destination(), Some(secret()), false).await,
            Err(IpcError::InvalidRequest)
        );
        let d = s
            .save_alert(destination(), Some(secret()), true)
            .await
            .unwrap();
        assert_eq!(fake.calls.load(Ordering::SeqCst), 0);
        let response = serde_json::to_string(&s.list_alerts().await.unwrap()).unwrap();
        assert!(!response.contains("private-path"));
        assert!(!response.contains("synthetic-sensitive"));
        assert_eq!(
            s.test_alert("d".into(), d.revision, false).await,
            Err(IpcError::InvalidRequest)
        );
        assert_eq!(
            s.test_alert("d".into(), 0, true).await,
            Err(IpcError::InvalidRequest)
        );
        s.test_alert("d".into(), d.revision, true).await.unwrap();
        s.process_alerts().await.unwrap();
        assert_eq!(fake.calls.load(Ordering::SeqCst), 1);
        assert_eq!(s.alert_history().await.unwrap()[0].status, "Accepted");
        let old = s.store.alert_secret_key("d").unwrap().unwrap();
        let mut disabled = d;
        disabled.enabled = false;
        let d = s.save_alert(disabled, None, false).await.unwrap();
        assert!(matches!(
            s.vault.get_secret(&key(&old).unwrap()),
            Err(vault::VaultError::NotFound(_))
        ));
        let current = s.store.alert_secret_key("d").unwrap().unwrap();
        s.delete_alert("d".into(), d.revision).await.unwrap();
        assert!(matches!(
            s.vault.get_secret(&key(&current).unwrap()),
            Err(vault::VaultError::NotFound(_))
        ));
    }
    #[tokio::test]
    async fn connectors_preserve_vault_consent_revision_and_queue_lifecycle() {
        for kind in [DestinationKind::Telegram, DestinationKind::Discord] {
            let (s, fake) = fixture();
            let mut d = destination();
            d.kind = kind;
            d.public_origin = Some(
                match kind {
                    DestinationKind::Telegram => "https://api.telegram.org",
                    _ => "https://discord.com",
                }
                .into(),
            );
            let secret = if kind == DestinationKind::Telegram {
                serde_json::from_str::<IncomingWebhook>(
                    r#"{"token":"123:synthetic","chat_id":"-123"}"#,
                )
                .unwrap()
            } else {
                IncomingWebhook(WebhookSecret {
                    url: "https://discord.com/api/webhooks/123/synthetic".into(),
                    bearer: None,
                })
            };
            let d = s.save_alert(d, Some(secret), true).await.unwrap();
            assert_eq!(fake.calls.load(Ordering::SeqCst), 0);
            let safe = serde_json::to_string(&s.list_alerts().await.unwrap()).unwrap();
            assert!(!safe.contains("synthetic"));
            assert!(!safe.contains("-123"));
            assert!(d.has_credentials);
            s.test_alert(d.id.clone(), d.revision, true).await.unwrap();
            let mut changed = d.clone();
            changed.kind = DestinationKind::Webhook;
            assert!(s.save_alert(changed, None, true).await.is_err());
            let updated = s.save_alert(d, None, true).await.unwrap();
            assert_eq!(s.alert_history().await.unwrap()[0].status, "Canceled");
            assert!(s
                .test_alert(updated.id.clone(), updated.revision - 1, true)
                .await
                .is_err());
            s.test_alert(updated.id.clone(), updated.revision, true)
                .await
                .unwrap();
            s.process_alerts().await.unwrap();
            assert_eq!(fake.calls.load(Ordering::SeqCst), 1);
            let mut disabled = updated;
            disabled.enabled = false;
            let disabled = s.save_alert(disabled, None, false).await.unwrap();
            assert!(s
                .test_alert(disabled.id.clone(), disabled.revision, true)
                .await
                .is_err());
            s.delete_alert(disabled.id, disabled.revision)
                .await
                .unwrap();
        }
    }
    #[tokio::test]
    async fn orphan_secret_intent_recovers_and_failed_save_keeps_previous_revision() {
        let (s, _) = fixture();
        let d = s
            .save_alert(destination(), Some(secret()), true)
            .await
            .unwrap();
        let orphan = "orphan";
        s.store.register_alert_secret(orphan).unwrap();
        s.vault
            .set_secret(&key(orphan).unwrap(), b"synthetic")
            .unwrap();
        let invalid = IncomingWebhook(WebhookSecret {
            url: "http://example.com/".into(),
            bearer: None,
        });
        assert_eq!(
            s.save_alert(d.clone(), Some(invalid), true).await,
            Err(IpcError::InvalidRequest)
        );
        assert_eq!(s.list_alerts().await.unwrap(), [d]);
        assert!(s.store.orphan_alert_secrets().unwrap().is_empty());
        assert!(matches!(
            s.vault.get_secret(&key(orphan).unwrap()),
            Err(vault::VaultError::NotFound(_))
        ));
    }
}
