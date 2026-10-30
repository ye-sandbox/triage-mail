use serde::{Deserialize, Serialize};
use std::fmt;

use crate::VaultError;

/// Transparent multi-mode authentication material stored securely in the vault.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AccountAuthMaterial {
    /// Standard IMAP password or provider-specific App Password.
    AppPassword { secret: String },
    /// Public OAuth2 PKCE tokens (`client_id` only, no secret required).
    OAuth2PublicPkce {
        client_id: String,
        access_token: String,
        refresh_token: Option<String>,
    },
    /// User-supplied Bring-Your-Own OAuth2 credentials (`client_id` + `client_secret`).
    OAuth2ByoCredentials {
        client_id: String,
        client_secret: String,
        access_token: String,
        refresh_token: Option<String>,
    },
    /// Local Proton Mail Bridge loopback password (`127.0.0.1`).
    ProtonBridgeLocal { bridge_password: String },
}

impl AccountAuthMaterial {
    /// Serializes auth material to binary JSON for vault storage.
    pub fn to_bytes(&self) -> Result<Vec<u8>, VaultError> {
        serde_json::to_vec(self)
            .map_err(|err| VaultError::Serialization(format!("serialization failed: {err}")))
    }

    /// Deserializes auth material from stored vault bytes.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, VaultError> {
        serde_json::from_slice(bytes)
            .map_err(|err| VaultError::Serialization(format!("deserialization failed: {err}")))
    }
}

// INVARIANT: Redact all raw secrets/passwords/tokens in Debug output to prevent accidental leakage in logs/panics.
impl fmt::Debug for AccountAuthMaterial {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AppPassword { .. } => f
                .debug_struct("AppPassword")
                .field("secret", &"[REDACTED]")
                .finish(),
            Self::OAuth2PublicPkce {
                client_id,
                refresh_token,
                ..
            } => f
                .debug_struct("OAuth2PublicPkce")
                .field("client_id", client_id)
                .field("access_token", &"[REDACTED]")
                .field(
                    "refresh_token",
                    &refresh_token.as_ref().map(|_| "[REDACTED]"),
                )
                .finish(),
            Self::OAuth2ByoCredentials {
                client_id,
                refresh_token,
                ..
            } => f
                .debug_struct("OAuth2ByoCredentials")
                .field("client_id", client_id)
                .field("client_secret", &"[REDACTED]")
                .field("access_token", &"[REDACTED]")
                .field(
                    "refresh_token",
                    &refresh_token.as_ref().map(|_| "[REDACTED]"),
                )
                .finish(),
            Self::ProtonBridgeLocal { .. } => f
                .debug_struct("ProtonBridgeLocal")
                .field("bridge_password", &"[REDACTED]")
                .finish(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_serialization_for_all_variants() {
        let fixtures = vec![
            AccountAuthMaterial::AppPassword {
                secret: "super-app-secret-password".into(),
            },
            AccountAuthMaterial::OAuth2PublicPkce {
                client_id: "public-client-id-example".into(),
                access_token: "ya29.synthetic-access-token".into(),
                refresh_token: Some("1//synthetic-refresh-token".into()),
            },
            AccountAuthMaterial::OAuth2ByoCredentials {
                client_id: "custom-byo-client-id".into(),
                client_secret: "custom-byo-client-secret-999".into(),
                access_token: "byo-access-token".into(),
                refresh_token: None,
            },
            AccountAuthMaterial::ProtonBridgeLocal {
                bridge_password: "bridge-local-pass-123".into(),
            },
        ];

        for item in fixtures {
            let bytes = item.to_bytes().expect("serializes cleanly");
            let deserialized =
                AccountAuthMaterial::from_bytes(&bytes).expect("deserializes cleanly");
            assert_eq!(item, deserialized);
        }
    }

    #[test]
    fn debug_formatting_strictly_redacts_all_secrets() {
        let app_pass = AccountAuthMaterial::AppPassword {
            secret: "SECRET_PASSWORD_DO_NOT_LEAK".into(),
        };
        let pkce = AccountAuthMaterial::OAuth2PublicPkce {
            client_id: "public_client_123".into(),
            access_token: "ACCESS_TOKEN_SECRET_XYZ".into(),
            refresh_token: Some("REFRESH_TOKEN_SECRET_ABC".into()),
        };
        let byo = AccountAuthMaterial::OAuth2ByoCredentials {
            client_id: "byo_client_456".into(),
            client_secret: "BYO_CLIENT_SECRET_789".into(),
            access_token: "BYO_ACCESS_TOKEN_SECRET_012".into(),
            refresh_token: Some("BYO_REFRESH_TOKEN_SECRET_345".into()),
        };
        let proton = AccountAuthMaterial::ProtonBridgeLocal {
            bridge_password: "PROTON_BRIDGE_PASSWORD_678".into(),
        };

        for (material, secrets) in [
            (app_pass, vec!["SECRET_PASSWORD_DO_NOT_LEAK"]),
            (
                pkce,
                vec!["ACCESS_TOKEN_SECRET_XYZ", "REFRESH_TOKEN_SECRET_ABC"],
            ),
            (
                byo,
                vec![
                    "BYO_CLIENT_SECRET_789",
                    "BYO_ACCESS_TOKEN_SECRET_012",
                    "BYO_REFRESH_TOKEN_SECRET_345",
                ],
            ),
            (proton, vec!["PROTON_BRIDGE_PASSWORD_678"]),
        ] {
            let debug_repr = format!("{material:?}");
            for secret in secrets {
                assert!(
                    !debug_repr.contains(secret),
                    "Debug representation leaked secret '{secret}': {debug_repr}"
                );
            }
            assert!(debug_repr.contains("[REDACTED]"));
        }
    }
}
