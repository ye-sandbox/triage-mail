//! Routed account aliases (Cloudflare Email Routing / forwarding) stored additively per account.
//!
//! INVARIANT: Aliases are local metadata in `config_json`; they never trigger network access.
use std::collections::HashMap;
use std::sync::Arc;

use mail_engine::AuthMethod;
use serde::{Deserialize, Serialize};
use storage::desktop::{normalize_aliases, AccountAlias};

use crate::service::storage_error;
use crate::{DesktopService, IpcError};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountAliasDto {
    pub address: String,
    #[serde(default)]
    pub label: Option<String>,
}

impl From<AccountAlias> for AccountAliasDto {
    fn from(a: AccountAlias) -> Self {
        Self {
            address: a.address,
            label: a.label,
        }
    }
}

impl From<AccountAliasDto> for AccountAlias {
    fn from(a: AccountAliasDto) -> Self {
        Self {
            address: a.address,
            label: a.label,
        }
    }
}

impl DesktopService {
    pub async fn account_aliases(
        self: &Arc<Self>,
    ) -> Result<HashMap<String, Vec<AccountAliasDto>>, IpcError> {
        let accounts = self.list_accounts().await?;
        self.blocking(move |s| {
            let mut out = HashMap::new();
            for config in accounts {
                if config.auth_method == AuthMethod::LocalArchive {
                    continue;
                }
                let list = s
                    .store
                    .account_aliases(&config.account_id)
                    .map_err(storage_error)?;
                if !list.is_empty() {
                    out.insert(
                        config.account_id,
                        list.into_iter().map(Into::into).collect(),
                    );
                }
            }
            Ok(out)
        })
        .await
    }

    pub async fn set_account_aliases(
        self: &Arc<Self>,
        account_id: String,
        aliases: Vec<AccountAliasDto>,
    ) -> Result<Vec<AccountAliasDto>, IpcError> {
        let aliases = normalize_aliases(aliases.into_iter().map(Into::into).collect())
            .ok_or(IpcError::InvalidRequest)?;
        let config = self.account(account_id).await?;
        if config.auth_method == AuthMethod::LocalArchive {
            return Err(IpcError::Unsupported);
        }
        self.blocking(move |s| {
            let saved = s
                .store
                .set_account_aliases(&config.account_id, aliases)
                .map_err(storage_error)?
                .ok_or(IpcError::NotFound)?;
            Ok(saved.into_iter().map(Into::into).collect())
        })
        .await
    }
}
