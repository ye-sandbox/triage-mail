//! Explicit, native-owned OAuth onboarding. No token or callback URL crosses back to the UI.
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;
use mail_engine::auth::{generate_oauth_state, LoopbackRedirectListener, PkceChallenge};
use mail_engine::AuthMethod;
use serde::{Deserialize, Serialize};
use tokio::sync::watch;
use url::Url;
use vault::{AccountAuthMaterial, VaultKey};
use zeroize::{Zeroize, Zeroizing};

use crate::service::{opaque_id, validate_config, IncomingCredentials};
use crate::{AccountConfigDto, DesktopService, IpcError};

#[derive(Serialize, Deserialize)]
struct AuthorizationFields {
    authorization_endpoint: String,
    token_endpoint: String,
    scopes: String,
    client_id: String,
    client_secret: Option<String>,
}
impl Drop for AuthorizationFields {
    fn drop(&mut self) {
        self.client_id.zeroize();
        self.client_secret.zeroize();
    }
}

/// Input-only wrapper prevents serde diagnostics from echoing credentials.
pub struct IncomingAuthorization(AuthorizationFields);
impl<'de> Deserialize<'de> for IncomingAuthorization {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        AuthorizationFields::deserialize(deserializer)
            .map(Self)
            .map_err(|_| serde::de::Error::custom("invalid authorization material"))
    }
}

#[derive(Serialize, Deserialize)]
struct ReauthorizationFields {
    account_id: String,
    authorization_endpoint: String,
    token_endpoint: Option<String>,
    scopes: String,
    client_id: Option<String>,
    client_secret: Option<String>,
}
impl Drop for ReauthorizationFields {
    fn drop(&mut self) {
        self.client_id.zeroize();
        self.client_secret.zeroize();
    }
}

/// Input-only wrapper prevents serde diagnostics from echoing reauthorization credentials.
pub struct IncomingReauthorization(ReauthorizationFields);
impl<'de> Deserialize<'de> for IncomingReauthorization {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        ReauthorizationFields::deserialize(deserializer)
            .map(Self)
            .map_err(|_| serde::de::Error::custom("invalid reauthorization material"))
    }
}

#[derive(Default)]
pub struct AuthorizationSessions(Mutex<HashMap<String, watch::Sender<bool>>>);

pub struct AuthorizationLease<'a> {
    sessions: &'a AuthorizationSessions,
    window: String,
}
impl Drop for AuthorizationLease<'_> {
    fn drop(&mut self) {
        if let Ok(mut sessions) = self.sessions.0.lock() {
            sessions.remove(&self.window);
        }
    }
}
impl AuthorizationSessions {
    pub fn begin(
        &self,
        window: &str,
    ) -> Result<(AuthorizationLease<'_>, watch::Receiver<bool>), IpcError> {
        let mut sessions = self.0.lock().map_err(|_| IpcError::Internal)?;
        if sessions.contains_key(window) {
            return Err(IpcError::InvalidRequest);
        }
        let (sender, receiver) = watch::channel(false);
        sessions.insert(window.into(), sender);
        Ok((
            AuthorizationLease {
                sessions: self,
                window: window.into(),
            },
            receiver,
        ))
    }
    pub fn cancel(&self, window: &str) {
        if let Ok(sessions) = self.0.lock() {
            if let Some(sender) = sessions.get(window) {
                sender.send_replace(true);
            }
        }
    }
}

fn endpoint(value: &str) -> Result<Url, IpcError> {
    let url = Url::parse(value).map_err(|_| IpcError::InvalidRequest)?;
    if value.len() > 2048
        || url.scheme() != "https"
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.fragment().is_some()
        || url.query().is_some()
    {
        return Err(IpcError::InvalidRequest);
    }
    Ok(url)
}

fn validate(config: &AccountConfigDto, fields: &AuthorizationFields) -> Result<(), IpcError> {
    endpoint(&fields.authorization_endpoint)?;
    endpoint(&fields.token_endpoint)?;
    if fields.scopes.is_empty()
        || fields.scopes.len() > 4096
        || fields.scopes.chars().any(char::is_control)
        || fields.client_id.len() > 4096
        || fields
            .client_secret
            .as_ref()
            .is_some_and(|s| s.len() > 8192)
    {
        return Err(IpcError::InvalidRequest);
    }
    let material = credentials(config.auth_method, fields, String::from("validation"), None)?;
    validate_config(config, &material.0)
}

/// Deterministic staging vault key for pending OAuth flows.
pub fn staging_oauth_vault_key() -> Result<VaultKey, IpcError> {
    VaultKey::for_account("staging", "oauth-pending").map_err(|_| IpcError::Internal)
}

/// Cleans any leftover orphaned OAuth staging entry from the vault.
pub fn clean_orphaned_staging(vault: &dyn vault::SecretStore) {
    if let Ok(key) = staging_oauth_vault_key() {
        let _ = vault.delete_secret(&key);
    }
}

fn credentials(
    method: AuthMethod,
    fields: &AuthorizationFields,
    access_token: String,
    refresh_token: Option<String>,
) -> Result<IncomingCredentials, IpcError> {
    Ok(IncomingCredentials(match method {
        AuthMethod::OAuth2PublicPkce if fields.client_secret.is_none() => {
            AccountAuthMaterial::OAuth2PublicPkce {
                client_id: fields.client_id.clone(),
                access_token,
                refresh_token,
                token_endpoint: Some(fields.token_endpoint.clone()),
            }
        }
        AuthMethod::OAuth2ByoCredentials => AccountAuthMaterial::OAuth2ByoCredentials {
            client_id: fields.client_id.clone(),
            client_secret: fields
                .client_secret
                .clone()
                .ok_or(IpcError::InvalidRequest)?,
            access_token,
            refresh_token,
            token_endpoint: Some(fields.token_endpoint.clone()),
        },
        _ => return Err(IpcError::InvalidRequest),
    }))
}

async fn run_pkce_flow(
    service: &Arc<DesktopService>,
    config: &AccountConfigDto,
    fields: AuthorizationFields,
    mut cancel: watch::Receiver<bool>,
    open: impl FnOnce(&str) -> Result<(), IpcError>,
    exchange: &dyn TokenExchange,
) -> Result<IncomingCredentials, IpcError> {
    validate(config, &fields)?;
    let listener = LoopbackRedirectListener::bind()
        .await
        .map_err(|_| IpcError::RemoteUnavailable)?;
    let redirect = listener.redirect_uri();
    let state = Zeroizing::new(generate_oauth_state());
    let pkce = PkceChallenge::generate();
    let verifier = Zeroizing::new(pkce.verifier.expose_secret().to_owned());
    let mut url = endpoint(&fields.authorization_endpoint)?;
    url.query_pairs_mut().extend_pairs([
        ("response_type", "code"),
        ("client_id", &fields.client_id),
        ("redirect_uri", &redirect),
        ("scope", &fields.scopes),
        ("state", &state),
        ("code_challenge", &pkce.challenge),
        ("code_challenge_method", "S256"),
    ]);
    // INVARIANT: Google OAuth2 omits refresh_token unless access_type=offline is requested.
    if url.host_str() == Some("accounts.google.com") {
        url.query_pairs_mut()
            .extend_pairs([("access_type", "offline"), ("prompt", "consent")]);
    }
    let key = staging_oauth_vault_key()?;
    let staged_key = key.clone();
    service
        .blocking(move |s| {
            let bytes =
                Zeroizing::new(serde_json::to_vec(&fields).map_err(|_| IpcError::Internal)?);
            s.vault
                .set_secret(&staged_key, &bytes)
                .map_err(|_| IpcError::VaultUnavailable)
        })
        .await?;
    let auth_method = config.auth_method;
    // INVARIANT: Cancellation only races callback/exchange. Once persistence starts, it completes.
    let result = tokio::select! {
        biased;
        _ = cancel.wait_for(|cancelled| *cancelled) => Err(IpcError::OAuthCancelled),
        result = tokio::time::timeout(Duration::from_secs(300), async {
            open(url.as_str())?;
            let code = Zeroizing::new(
                listener
                    .wait_for_code(&state, Duration::from_secs(300))
                    .await
                    .map_err(|err| {
                        err.diagnostic_code()
                            .map(crate::service::diagnostic_to_ipc)
                            .unwrap_or(IpcError::RemoteUnavailable)
                    })?,
            );
            let read_key = key.clone();
            let fields = service.blocking(move |s| {
                let bytes = Zeroizing::new(s.vault.get_secret(&read_key).map_err(|_| IpcError::VaultUnavailable)?);
                serde_json::from_slice::<AuthorizationFields>(&bytes).map_err(|_| IpcError::VaultUnavailable)
            }).await?;
            let tokens = exchange.exchange(TokenRequest { target: &fields.token_endpoint, client_id: &fields.client_id, client_secret: fields.client_secret.as_deref(), redirect: &redirect, code: &code, verifier: &verifier }).await?;
            credentials(auth_method, &fields, tokens.access_token.clone(), tokens.refresh_token.clone())
        }) => result.map_err(|_| IpcError::NetworkTimeout).and_then(|r| r),
    };
    service
        .blocking(move |s| {
            s.vault
                .delete_secret(&key)
                .map_err(|_| IpcError::VaultUnavailable)
        })
        .await?;
    result
}

/// All blocking keyring work runs through the service executor. Staging ends on every exit path.
pub async fn authorize(
    service: Arc<DesktopService>,
    mut config: AccountConfigDto,
    input: IncomingAuthorization,
    cancel: watch::Receiver<bool>,
    open: impl FnOnce(&str) -> Result<(), IpcError>,
    exchange: &dyn TokenExchange,
) -> Result<String, IpcError> {
    if config.account_id.is_empty() {
        config.account_id = opaque_id()?;
    }
    let creds = run_pkce_flow(&service, &config, input.0, cancel, open, exchange).await?;
    service.add_account(config, creds).await
}

/// Re-runs OAuth2 PKCE consent for an existing account and replaces vault tokens in place.
pub async fn reauthorize(
    service: Arc<DesktopService>,
    mut input: IncomingReauthorization,
    cancel: watch::Receiver<bool>,
    open: impl FnOnce(&str) -> Result<(), IpcError>,
    exchange: &dyn TokenExchange,
) -> Result<String, IpcError> {
    let account_id = input.0.account_id.clone();
    if account_id.is_empty()
        || account_id.len() > 128
        || !account_id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
    {
        return Err(IpcError::InvalidRequest);
    }
    let mut stored = service
        .load_oauth_client_material(account_id.clone())
        .await?;
    let config = stored.config.clone();
    let override_client_id = input.0.client_id.take().filter(|v| !v.is_empty());
    let override_client_secret = input.0.client_secret.take().filter(|v| !v.is_empty());
    let override_token_endpoint = input.0.token_endpoint.take().filter(|v| !v.is_empty());
    let client_secret = match config.auth_method {
        AuthMethod::OAuth2PublicPkce => {
            if let Some(mut secret) = override_client_secret {
                secret.zeroize();
                return Err(IpcError::InvalidRequest);
            }
            None
        }
        AuthMethod::OAuth2ByoCredentials => Some(
            override_client_secret
                .or_else(|| stored.client_secret.take())
                .ok_or(IpcError::InvalidRequest)?,
        ),
        _ => return Err(IpcError::InvalidRequest),
    };
    let fields = AuthorizationFields {
        authorization_endpoint: std::mem::take(&mut input.0.authorization_endpoint),
        token_endpoint: override_token_endpoint
            .or_else(|| stored.token_endpoint.take())
            .ok_or(IpcError::InvalidRequest)?,
        scopes: std::mem::take(&mut input.0.scopes),
        client_id: override_client_id.unwrap_or_else(|| std::mem::take(&mut stored.client_id)),
        client_secret,
    };
    drop(stored);
    drop(input);
    let creds = run_pkce_flow(&service, &config, fields, cancel, open, exchange).await?;
    service.update_account_auth(config, creds).await?;
    Ok(account_id)
}

#[derive(Deserialize)]
pub struct TokenResponse {
    access_token: String,
    refresh_token: Option<String>,
    token_type: String,
}
impl Drop for TokenResponse {
    fn drop(&mut self) {
        self.access_token.zeroize();
        self.refresh_token.zeroize();
    }
}

pub struct TokenRequest<'a> {
    target: &'a str,
    client_id: &'a str,
    client_secret: Option<&'a str>,
    redirect: &'a str,
    code: &'a str,
    verifier: &'a str,
}
#[async_trait]
pub trait TokenExchange: Send + Sync {
    async fn exchange(&self, request: TokenRequest<'_>) -> Result<TokenResponse, IpcError>;
}
pub struct NativeTokenExchange;
#[async_trait]
impl TokenExchange for NativeTokenExchange {
    async fn exchange(&self, request: TokenRequest<'_>) -> Result<TokenResponse, IpcError> {
        let TokenRequest {
            target,
            client_id,
            client_secret,
            redirect,
            code,
            verifier,
        } = request;
        let mut form = vec![
            ("grant_type", "authorization_code"),
            ("client_id", client_id),
            ("redirect_uri", redirect),
            ("code", code),
            ("code_verifier", verifier),
        ];
        if let Some(secret) = client_secret {
            form.push(("client_secret", secret));
        }
        let client = reqwest::Client::builder()
            .https_only(true)
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|_| IpcError::RemoteUnavailable)?;
        let mut response = client
            .post(endpoint(target)?)
            .form(&form)
            .send()
            .await
            .map_err(|err| {
                if err.is_timeout() {
                    IpcError::NetworkTimeout
                } else {
                    IpcError::RemoteUnavailable
                }
            })?;
        if !response.status().is_success() {
            let status_code = response.status().as_u16();
            let mut error_bytes = Zeroizing::new(Vec::new());
            while let Ok(Some(chunk)) = response.chunk().await {
                if error_bytes.len() + chunk.len() > 16384 {
                    break;
                }
                error_bytes.extend_from_slice(&chunk);
            }
            let engine_err =
                mail_engine::auth::classify_token_error_response(status_code, &error_bytes);
            return Err(engine_err
                .diagnostic_code()
                .map(crate::service::diagnostic_to_ipc)
                .unwrap_or(IpcError::RemoteUnavailable));
        }
        // INVARIANT: Provider responses are bounded, zeroized, and never included in diagnostics.
        let mut bytes = Zeroizing::new(Vec::new());
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|_| IpcError::RemoteUnavailable)?
        {
            if bytes.len() + chunk.len() > 65536 {
                return Err(IpcError::RemoteUnavailable);
            }
            bytes.extend_from_slice(&chunk);
        }
        parse_tokens(&bytes)
    }
}
fn parse_tokens(bytes: &[u8]) -> Result<TokenResponse, IpcError> {
    let tokens: TokenResponse =
        serde_json::from_slice(bytes).map_err(|_| IpcError::RemoteUnavailable)?;
    if !tokens.token_type.eq_ignore_ascii_case("bearer")
        || tokens.access_token.is_empty()
        || tokens.access_token.chars().any(char::is_control)
        || tokens
            .refresh_token
            .as_ref()
            .is_some_and(|v| v.is_empty() || v.chars().any(char::is_control))
    {
        return Err(IpcError::RemoteUnavailable);
    }
    Ok(tokens)
}

/// Fixed executable and argument boundaries; never invoke a shell with provider-supplied text.
pub fn open_browser(url: &str) -> Result<(), IpcError> {
    use std::process::{Command, Stdio};
    #[cfg(target_os = "linux")]
    let mut command = Command::new("xdg-open");
    #[cfg(target_os = "macos")]
    let mut command = Command::new("open");
    #[cfg(target_os = "windows")]
    let mut command = {
        let mut command = Command::new("rundll32.exe");
        command.arg("url.dll,FileProtocolHandler");
        command
    };
    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    return Err(IpcError::Unsupported);
    #[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
    {
        let mut child = command
            .arg(url)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|_| IpcError::RemoteUnavailable)?;
        std::thread::spawn(move || {
            let _ = child.wait();
        });
        Ok(())
    }
}

#[cfg(test)]
mod tests;

/// Secret-free discovery does not initialize the vault or start sync workers.
pub async fn discover_provider(
    email: &str,
    online: bool,
    transport: &dyn mail_engine::discovery::DiscoveryTransport,
) -> Result<crate::DiscoveredProviderDto, IpcError> {
    mail_engine::discovery::discover(email, online, transport)
        .await
        .map_err(discovery_error)
}

pub async fn probe_provider(
    host: &str,
    transport: &dyn mail_engine::discovery::DiscoveryTransport,
) -> Result<bool, IpcError> {
    mail_engine::discovery::probe_confirmed_host(host, transport)
        .await
        .map_err(discovery_error)
}

fn discovery_error(error: mail_engine::discovery::DiscoveryError) -> IpcError {
    match error {
        mail_engine::discovery::DiscoveryError::InvalidInput => IpcError::InvalidRequest,
        mail_engine::discovery::DiscoveryError::Unavailable => IpcError::RemoteUnavailable,
    }
}
