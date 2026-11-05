use super::*;
use crate::DatabaseKey;
fn store() -> SqliteMailStore {
    let s = SqliteMailStore::open_in_memory(&DatabaseKey::from_bytes([2; 32]), Default::default())
        .unwrap();
    s.conn
        .lock()
        .unwrap()
        .execute(
            "INSERT INTO accounts VALUES ('a','reader@example.com','AppPassword',0,0)",
            [],
        )
        .unwrap();
    s
}
fn rule() -> Rule {
    Rule {
        version: 1,
        id: "source-rule".into(),
        revision: 0,
        name: "Synthetic".into(),
        enabled: true,
        position: 0,
        all: vec![Condition::Account("a".into())],
        any: vec![],
        except: vec![],
        actions: vec![RuleAction::Star],
        stop_processing: false,
    }
}
#[test]
fn roundtrip_remaps_fresh_ids_and_never_queues_history() {
    let source = store();
    source.save_rule(rule()).unwrap();
    let bundle = source.export_definitions().unwrap();
    assert_eq!(bundle.accounts, vec!["account-1"]);
    assert!(!bundle.rules[0].enabled);
    let target = store();
    assert!(target
        .import_definitions(bundle.clone(), ReferenceMapping::default())
        .is_err());
    target
        .import_definitions(
            bundle,
            ReferenceMapping {
                accounts: BTreeMap::from([("account-1".into(), "a".into())]),
                destinations: BTreeMap::new(),
            },
        )
        .unwrap();
    let rules = target.list_rules().unwrap();
    assert_eq!(rules.len(), 1);
    assert_ne!(rules[0].id, "source-rule");
    assert!(!rules[0].enabled);
    assert_eq!(rules[0].revision, 1);
    assert_eq!(
        target
            .conn
            .lock()
            .unwrap()
            .query_row("SELECT count(*) FROM rule_jobs", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        0
    );
}
#[test]
fn unsupported_version_missing_targets_and_transaction_rollback() {
    let source = store();
    source.save_rule(rule()).unwrap();
    let mut bundle = source.export_definitions().unwrap();
    bundle.version = 99;
    assert!(PortableDefinitions::parse(&serde_json::to_vec(&bundle).unwrap()).is_err());
    bundle.version = 1;
    let target = store();
    assert!(target
        .import_definitions(
            bundle,
            ReferenceMapping {
                accounts: BTreeMap::from([("account-1".into(), "missing".into())]),
                destinations: BTreeMap::new()
            }
        )
        .is_err());
    assert!(target.list_rules().unwrap().is_empty());
}

#[test]
fn invalid_view_after_rule_insert_rolls_back_whole_import() {
    use crate::{
        saved_views::{SavedView, ViewFilters},
        MailboxScope, MailboxView, SearchMode,
    };
    let source = store();
    source.save_rule(rule()).unwrap();
    let mut bundle = source.export_definitions().unwrap();
    bundle.views.push(SavedView {
        version: 1,
        id: "view-1".into(),
        revision: 0,
        name: "Synthetic missing folder".into(),
        query: String::new(),
        mode: SearchMode::Lexical,
        accounts: vec!["account-1".into()],
        scope: Some(MailboxScope::AccountView {
            account_id: "account-1".into(),
            view: MailboxView::Folder("missing-folder".into()),
        }),
        filters: ViewFilters::default(),
    });
    let target = store();
    assert!(target
        .import_definitions(
            bundle,
            ReferenceMapping {
                accounts: BTreeMap::from([("account-1".into(), "a".into())]),
                destinations: BTreeMap::new()
            }
        )
        .is_err());
    assert!(target.list_rules().unwrap().is_empty());
    assert!(target.list_saved_views().unwrap().is_empty());
}
