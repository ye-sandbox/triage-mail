//! Account subscription badges: IMAP quota refresh, tier inference and manual overrides.
//!
//! INVARIANT: Quota refresh is best-effort and never fails message sync; badges derive solely
//! from the server-reported RFC 2087 quota or the user's explicit override.
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use mail_engine::{
    infer_subscription_badge, AccountBadgeOverride, AccountPlanBadge, AccountStorageQuota,
    AuthMethod, ProviderKind,
};
use serde::{Deserialize, Serialize};
use storage::desktop::AccountConnectionExtra;

use crate::service::storage_error;
use crate::{AccountConfigDto, DesktopService, IpcError};

/// Minimum interval between quota refreshes per account.
const QUOTA_REFRESH_INTERVAL: Duration = Duration::from_secs(6 * 60 * 60);

/// Subscription view of one account returned to the UI.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountSubscriptionDto {
    pub account_id: String,
    pub effective_badge: Option<String>,
    pub badge_override: AccountBadgeOverride,
    pub inferred_badge: Option<AccountPlanBadge>,
    pub quota: Option<AccountStorageQuota>,
}

fn preset_label(badge: AccountPlanBadge) -> &'static str {
    match badge {
        AccountPlanBadge::Ultra => "ULTRA",
        AccountPlanBadge::Pro => "PRO",
        AccountPlanBadge::One => "ONE",
        AccountPlanBadge::Workspace => "WORKSPACE",
        AccountPlanBadge::Plus => "PLUS",
    }
}

fn build_dto(
    config: &AccountConfigDto,
    quota: Option<AccountStorageQuota>,
    badge_override: AccountBadgeOverride,
) -> AccountSubscriptionDto {
    let provider = ProviderKind::detect(&config.imap_host, None);
    let inferred = infer_subscription_badge(provider, &config.imap_host, quota);
    let effective = match &badge_override {
        AccountBadgeOverride::Auto => inferred.map(|b| preset_label(b).to_string()),
        AccountBadgeOverride::Hidden => None,
        AccountBadgeOverride::Preset(b) => Some(preset_label(*b).to_string()),
        AccountBadgeOverride::Custom(label) => Some(label.clone()),
    };
    AccountSubscriptionDto {
        account_id: config.account_id.clone(),
        effective_badge: effective,
        badge_override,
        inferred_badge: inferred,
        quota,
    }
}

impl DesktopService {
    fn read_extra<T: serde::de::DeserializeOwned>(
        &self,
        account_id: &str,
        key: AccountConnectionExtra,
    ) -> Result<Option<T>, IpcError> {
        let raw = self
            .store
            .get_account_connection_extra(account_id, key)
            .map_err(storage_error)?;
        // Malformed extras degrade to "absent" rather than failing the account list.
        Ok(raw.and_then(|json| serde_json::from_str(&json).ok()))
    }

    pub async fn account_subscriptions(
        self: &Arc<Self>,
    ) -> Result<HashMap<String, AccountSubscriptionDto>, IpcError> {
        let accounts = self.list_accounts().await?;
        self.blocking(move |s| {
            let mut out = HashMap::new();
            for config in accounts {
                if config.auth_method == AuthMethod::LocalArchive {
                    continue;
                }
                let quota = s.read_extra(&config.account_id, AccountConnectionExtra::Quota)?;
                let over = s
                    .read_extra(&config.account_id, AccountConnectionExtra::BadgeOverride)?
                    .unwrap_or_default();
                out.insert(config.account_id.clone(), build_dto(&config, quota, over));
            }
            Ok(out)
        })
        .await
    }

    pub async fn set_account_badge(
        self: &Arc<Self>,
        account_id: String,
        badge_override: AccountBadgeOverride,
    ) -> Result<AccountSubscriptionDto, IpcError> {
        let badge_override = badge_override
            .normalized()
            .ok_or(IpcError::InvalidRequest)?;
        let config = self.account(account_id).await?;
        if config.auth_method == AuthMethod::LocalArchive {
            return Err(IpcError::Unsupported);
        }
        self.blocking(move |s| {
            let json = serde_json::to_string(&badge_override).map_err(|_| IpcError::Internal)?;
            let stored = s
                .store
                .set_account_connection_extra(
                    &config.account_id,
                    AccountConnectionExtra::BadgeOverride,
                    &json,
                )
                .map_err(storage_error)?;
            if !stored {
                return Err(IpcError::NotFound);
            }
            let quota = s.read_extra(&config.account_id, AccountConnectionExtra::Quota)?;
            Ok(build_dto(&config, quota, badge_override))
        })
        .await
    }

    /// Best-effort, throttled quota refresh; every failure is swallowed so sync is unaffected.
    pub async fn refresh_account_quota(self: &Arc<Self>, config: &AccountConfigDto) {
        if config.auth_method == AuthMethod::LocalArchive || !self.quota_refresh_due(config) {
            return;
        }
        let Ok(Some(quota)) = self.remote.fetch_quota(config).await else {
            return;
        };
        let account_id = config.account_id.clone();
        let _ = self
            .blocking(move |s| {
                let json = serde_json::to_string(&quota).map_err(|_| IpcError::Internal)?;
                s.store
                    .set_account_connection_extra(&account_id, AccountConnectionExtra::Quota, &json)
                    .map(|_| ())
                    .map_err(storage_error)
            })
            .await;
    }

    fn quota_refresh_due(&self, config: &AccountConfigDto) -> bool {
        let Ok(mut map) = self.quota_refreshed.lock() else {
            return false;
        };
        let now = Instant::now();
        match map.get(&config.account_id) {
            Some(last) if now.duration_since(*last) < QUOTA_REFRESH_INTERVAL => false,
            _ => {
                map.insert(config.account_id.clone(), now);
                true
            }
        }
    }
}
