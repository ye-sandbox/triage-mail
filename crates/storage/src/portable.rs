//! Credential-free rule/view interchange with explicit opaque reference mapping.
use std::collections::{BTreeMap, BTreeSet};

use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use triage_ai::rules::{Condition, Rule, RuleAction, MAX_RULES};

use crate::{backup::invalid, saved_views::SavedView, MailboxScope, SqliteMailStore, StorageError};

pub const MAX_PORTABLE_BYTES: usize = 4 * 1024 * 1024;
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PortableDefinitions {
    pub version: u32,
    pub rules: Vec<Rule>,
    pub views: Vec<SavedView>,
    pub accounts: Vec<String>,
    pub destinations: Vec<String>,
}
#[derive(Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReferenceMapping {
    pub accounts: BTreeMap<String, String>,
    pub destinations: BTreeMap<String, String>,
}
impl PortableDefinitions {
    pub fn parse(bytes: &[u8]) -> Result<Self, StorageError> {
        if bytes.len() > MAX_PORTABLE_BYTES {
            return Err(invalid());
        }
        let value: Self = serde_json::from_slice(bytes).map_err(|_| invalid())?;
        value.validate()?;
        Ok(value)
    }
    fn validate(&self) -> Result<(), StorageError> {
        let (accounts, destinations) = references(&self.rules, &self.views);
        if self.version != 1
            || self.rules.len() > MAX_RULES
            || self.views.len() > 200
            || !self.rules.iter().all(Rule::validate)
            || !self.views.iter().all(SavedView::validate)
            || accounts.into_iter().collect::<Vec<_>>() != self.accounts
            || destinations.into_iter().collect::<Vec<_>>() != self.destinations
            || self.accounts.len() > 100
            || self.destinations.len() > 200
        {
            return Err(invalid());
        }
        Ok(())
    }
}
fn references(rules: &[Rule], views: &[SavedView]) -> (BTreeSet<String>, BTreeSet<String>) {
    let mut accounts = BTreeSet::new();
    let mut destinations = BTreeSet::new();
    for rule in rules {
        for c in rule.all.iter().chain(&rule.any).chain(&rule.except) {
            if let Condition::Account(id) = c {
                accounts.insert(id.clone());
            }
        }
        for action in &rule.actions {
            if let RuleAction::Alert(id) | RuleAction::UrgentAlert(id) = action {
                destinations.insert(id.clone());
            }
        }
    }
    for view in views {
        accounts.extend(view.accounts.iter().cloned());
        if let Some(MailboxScope::AccountView { account_id, .. }) = &view.scope {
            accounts.insert(account_id.clone());
        }
    }
    (accounts, destinations)
}
fn remap(
    rules: &mut [Rule],
    views: &mut [SavedView],
    mapping: &ReferenceMapping,
) -> Result<(), StorageError> {
    for rule in rules {
        for c in rule
            .all
            .iter_mut()
            .chain(&mut rule.any)
            .chain(&mut rule.except)
        {
            if let Condition::Account(id) = c {
                *id = mapping.accounts.get(id).ok_or_else(invalid)?.clone();
            }
        }
        for action in &mut rule.actions {
            if let RuleAction::Alert(id) | RuleAction::UrgentAlert(id) = action {
                *id = mapping.destinations.get(id).ok_or_else(invalid)?.clone();
            }
        }
        rule.enabled = false;
    }
    for view in views {
        for id in &mut view.accounts {
            *id = mapping.accounts.get(id).ok_or_else(invalid)?.clone();
        }
        if let Some(MailboxScope::AccountView { account_id, .. }) = &mut view.scope {
            *account_id = mapping
                .accounts
                .get(account_id)
                .ok_or_else(invalid)?
                .clone();
        }
    }
    Ok(())
}
fn read_views(conn: &Connection) -> Result<Vec<SavedView>, StorageError> {
    let mut stmt = conn
        .prepare("SELECT definition_json FROM saved_views ORDER BY id")
        .map_err(|_| invalid())?;
    let values = stmt
        .query_map([], |r| r.get::<_, String>(0))
        .map_err(|_| invalid())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| invalid())?;
    values
        .into_iter()
        .map(|v| serde_json::from_str(&v).map_err(|_| invalid()))
        .collect()
}
impl SqliteMailStore {
    pub fn export_definitions(&self) -> Result<PortableDefinitions, StorageError> {
        self.transaction(|tx| {
            let mut rules = crate::rules::read_rules(tx)?;
            let mut views = read_views(tx)?;
            let (accounts, destinations) = references(&rules, &views);
            let mapping = ReferenceMapping {
                accounts: accounts
                    .into_iter()
                    .enumerate()
                    .map(|(i, id)| (id, format!("account-{}", i + 1)))
                    .collect(),
                destinations: destinations
                    .into_iter()
                    .enumerate()
                    .map(|(i, id)| (id, format!("destination-{}", i + 1)))
                    .collect(),
            };
            remap(&mut rules, &mut views, &mapping)?;
            for (i, rule) in rules.iter_mut().enumerate() {
                rule.id = format!("rule-{}", i + 1);
                rule.revision = 0;
            }
            for (i, view) in views.iter_mut().enumerate() {
                view.id = format!("view-{}", i + 1);
                view.revision = 0;
            }
            let (accounts, destinations) = references(&rules, &views);
            let bundle = PortableDefinitions {
                version: 1,
                rules,
                views,
                accounts: accounts.into_iter().collect(),
                destinations: destinations.into_iter().collect(),
            };
            PortableDefinitions::parse(&serde_json::to_vec(&bundle).map_err(|_| invalid())?)
        })
    }
    pub fn import_definitions(
        &self,
        mut bundle: PortableDefinitions,
        mapping: ReferenceMapping,
    ) -> Result<(), StorageError> {
        bundle.validate()?;
        if mapping.accounts.keys().cloned().collect::<Vec<_>>() != bundle.accounts
            || mapping.destinations.keys().cloned().collect::<Vec<_>>() != bundle.destinations
        {
            return Err(invalid());
        }
        remap(&mut bundle.rules, &mut bundle.views, &mapping)?;
        self.write_transaction(
            |tx| {
                for account in mapping.accounts.values() {
                    if !tx
                        .query_row(
                            "SELECT EXISTS(SELECT 1 FROM accounts WHERE id=?1)",
                            [account],
                            |r| r.get::<_, bool>(0),
                        )
                        .map_err(|_| invalid())?
                    {
                        return Err(invalid());
                    }
                }
                let destinations = crate::alerts::destinations(tx)?;
                if mapping
                    .destinations
                    .values()
                    .any(|id| !destinations.iter().any(|d| &d.id == id))
                {
                    return Err(invalid());
                }
                let existing = crate::rules::read_rules(tx)?.len();
                if existing + bundle.rules.len() > MAX_RULES
                    || read_views(tx)?.len() + bundle.views.len() > 200
                {
                    return Err(invalid());
                }
                for (i, mut rule) in bundle.rules.into_iter().enumerate() {
                    rule.id = vault::backup::random_id()?;
                    rule.revision = 1;
                    rule.position = (existing + i) as u32;
                    if !rule.validate() {
                        return Err(invalid());
                    }
                    let definition = serde_json::to_string(&rule).map_err(|_| invalid())?;
                    tx.execute(
                        "INSERT INTO local_rules VALUES (?1,1,?2,?3)",
                        params![rule.id, rule.position, definition],
                    )
                    .map_err(|_| invalid())?;
                }
                for mut view in bundle.views {
                    view.id = vault::backup::random_id()?;
                    view.revision = 1;
                    if !view.validate() {
                        return Err(invalid());
                    }
                    crate::saved_views::validate_accounts(tx, &view)?;
                    let definition = serde_json::to_string(&view).map_err(|_| invalid())?;
                    tx.execute(
                        "INSERT INTO saved_views VALUES (?1,1,?2)",
                        params![view.id, definition],
                    )
                    .map_err(|_| invalid())?;
                }
                Ok(())
            },
            |_, _, _| Ok(()),
        )
    }
}

#[cfg(test)]
mod tests;
