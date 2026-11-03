use super::*;
use crate::{DatabaseKey, HybridQuery, MailboxScope, MailboxView, SearchMode};
fn fixture() -> SqliteMailStore {
    let store =
        SqliteMailStore::open_in_memory(&DatabaseKey::from_bytes([91; 32]), Default::default())
            .unwrap();
    store.transaction(|tx| {tx.execute_batch("INSERT INTO accounts VALUES ('a','a@example.com','synthetic',0,0),('b','b@example.com','synthetic',0,0); INSERT INTO folders VALUES ('a','INBOX','Inbox','/','inbox',1,NULL,NULL,0),('a','Archive','Archive','/','archive',1,NULL,NULL,0);").map_err(error)?;Ok(())}).unwrap();
    store
}
fn insert(
    s: &SqliteMailStore,
    id: &str,
    account: &str,
    uid: u32,
    header: Option<&str>,
    parent: Option<&str>,
    folder: &str,
) {
    s.transaction(|tx| {tx.execute("INSERT INTO messages(id,account_id,folder,uid,message_id_header,in_reply_to,subject,sender,recipients,received_at_epoch_ms,raw_headers,created_at_epoch_ms) VALUES (?1,?2,?3,?4,?5,?6,'Same subject','sender@example.org','reader@example.com',?4,'',0)",params![id,account,folder,uid,header,parent]).map_err(error)?;
    tx.execute("INSERT INTO message_folder_memberships VALUES (?1,?2,?3,?4,0)",params![account,folder,uid,id]).map_err(error)?; Ok(())}).unwrap();
}
fn indexed(s: &SqliteMailStore) {
    for _ in 0..20 {
        s.backfill_conversations().unwrap();
        if s.conversation_progress().unwrap().ready {
            return;
        }
    }
    panic!("not ready");
}
fn query() -> HybridQuery {
    HybridQuery {
        query_text: "".into(),
        query_embedding_int8: None,
        account_filter: None,
        mode: SearchMode::Lexical,
        limit: 2,
        offset: 0,
    }
}
fn context() -> ConversationQuery {
    ConversationQuery {
        source: ConversationSource::Messages,
        scope: None,
        generation: None,
    }
}
#[test]
fn account_boundary_cross_folder_filter_pagination_preview_and_stale_guard() {
    let s = fixture();
    insert(&s, "root", "a", 1, Some("root@example.org"), None, "INBOX");
    insert(
        &s,
        "reply",
        "a",
        2,
        Some("reply@example.org"),
        Some("root@example.org"),
        "Archive",
    );
    insert(&s, "collision", "a", 3, None, None, "INBOX");
    insert(
        &s,
        "other-account",
        "b",
        1,
        Some("root@example.org"),
        None,
        "INBOX",
    );
    assert!(s.search_conversations(&query(), &context()).is_err());
    indexed(&s);
    let mut ctx = context();
    ctx.scope = Some(MailboxScope::AccountView {
        account_id: "a".into(),
        view: MailboxView::Inbox,
    });
    let result = s.search_conversations(&query(), &ctx).unwrap();
    assert_eq!(result.rows.len(), 2);
    let row = result.rows.iter().find(|r| r.total_count == 2).unwrap();
    assert_eq!(row.matching_count, 1);
    ctx.generation = Some(result.generation.clone());
    let members = s
        .conversation_members(&query(), &ctx, &row.conversation_id, 0)
        .unwrap();
    assert_eq!(members.total, 2);
    assert!(!members.rows[1].matching);
    let preview = s
        .preview_conversation(
            &query(),
            &ctx,
            &row.conversation_id,
            false,
            ConversationAction::MarkRead,
        )
        .unwrap();
    assert_eq!(preview.messages.len(), 1);
    assert_eq!(preview.outside_count, 0);
    let all = s
        .preview_conversation(
            &query(),
            &ctx,
            &row.conversation_id,
            true,
            ConversationAction::Tag("review".into()),
        )
        .unwrap();
    assert_eq!(all.outside_count, 1);
    let (generation, ids, _) = s.conversation_preview(&all.token).unwrap();
    insert(&s, "new", "a", 4, None, Some("root@example.org"), "INBOX");
    assert!(s
        .apply_local_mutations_guarded(
            &[(
                ids[0].clone(),
                crate::desktop::LocalMutation::Read(true),
                None
            )],
            Some(&generation)
        )
        .is_err());
    assert!(s.conversation_preview(&preview.token).is_err());
    assert!(s.search_conversations(&query(), &ctx).is_err());
    indexed(&s);
    ctx.generation = None;
    assert_eq!(
        s.search_conversations(&query(), &ctx)
            .unwrap()
            .rows
            .iter()
            .find(|r| r.total_count == 3)
            .unwrap()
            .matching_count,
        2
    );
    s.transaction(|tx| {
        tx.execute("DELETE FROM accounts WHERE id='a'", [])
            .map_err(error)?;
        Ok(())
    })
    .unwrap();
    indexed(&s);
    assert_eq!(
        s.search_conversations(&query(), &context())
            .unwrap()
            .rows
            .len(),
        1
    );
}
#[test]
fn graph_ambiguity_missing_parents_cycles_and_strict_ids() {
    use graph::{group, header_id, references, Input};
    let node = |id: &str, h: Option<&str>, p: Option<&str>, refs: Vec<String>| Input {
        id: id.into(),
        header: h.map(str::to_string),
        parent: p.map(str::to_string),
        refs,
    };
    let nodes = vec![
        node("a", Some("dup@example.org"), None, vec![]),
        node("b", Some("dup@example.org"), None, vec![]),
        node("c", None, Some("dup@example.org"), vec![]),
        node("d", None, Some("missing@example.org"), vec![]),
        node("e", None, None, vec!["missing@example.org".into()]),
        node("f", Some("f@example.org"), Some("g@example.org"), vec![]),
        node("g", Some("g@example.org"), Some("f@example.org"), vec![]),
    ];
    let groups: std::collections::HashMap<_, _> = group(&nodes).into_iter().collect();
    assert_ne!(groups["a"], groups["b"]);
    assert_ne!(groups["a"], groups["c"]);
    assert_eq!(groups["d"], groups["e"]);
    assert_eq!(groups["f"], groups["g"]);
    assert_ne!(header_id("Case@example.org"), header_id("case@example.org"));
    for id in [
        "",
        "bad",
        "<x@example.org",
        "a b@example.org",
        "x@@example.org",
        "<x@example.org> junk",
    ] {
        assert!(header_id(id).is_none(), "{id}");
    }
    assert!(references(&"<a@example.org> ".repeat(65)).is_empty());
    assert!(references("<a@example.org> malformed").is_empty());
    let long: Vec<_> = (0..10000)
        .map(|i| {
            node(
                &format!("{i:05}"),
                Some(&format!("{i}@example.org")),
                (i > 0).then(|| format!("{}@example.org", i - 1)).as_deref(),
                vec![],
            )
        })
        .collect();
    assert!(group(&long).iter().all(|(_, root)| root == "00000"));
}
#[test]
fn duplicate_appearance_deletion_and_atomic_rollback() {
    let s = fixture();
    insert(&s, "a", "a", 1, Some("root@example.org"), None, "INBOX");
    insert(&s, "b", "a", 2, None, Some("root@example.org"), "INBOX");
    indexed(&s);
    assert_eq!(
        s.search_conversations(&query(), &context())
            .unwrap()
            .rows
            .len(),
        1
    );
    insert(
        &s,
        "duplicate",
        "a",
        3,
        Some("root@example.org"),
        None,
        "Archive",
    );
    indexed(&s);
    let mut q = query();
    q.limit = 10;
    assert_eq!(
        s.search_conversations(&q, &context()).unwrap().rows.len(),
        3
    );
    s.transaction(|tx| {
        tx.execute("DELETE FROM messages WHERE id='duplicate'", [])
            .map_err(error)?;
        Ok(())
    })
    .unwrap();
    indexed(&s);
    assert_eq!(
        s.search_conversations(&query(), &context())
            .unwrap()
            .rows
            .len(),
        1
    );
    let changes = vec![
        ("a".into(), crate::desktop::LocalMutation::Read(true), None),
        (
            "absent".into(),
            crate::desktop::LocalMutation::Read(true),
            None,
        ),
    ];
    assert!(s.apply_local_mutations(&changes).is_err());
    assert!(
        !s.search_conversations(&query(), &context()).unwrap().rows[0]
            .representative
            .is_read
    );
}
#[test]
fn page_generation_and_oversized_actions_fail_closed() {
    let s = fixture();
    for i in 0..501 {
        insert(
            &s,
            &format!("m{i:04}"),
            "a",
            i + 1,
            None,
            Some("missing@example.org"),
            "INBOX",
        );
    }
    assert_eq!(s.backfill_conversations().unwrap(), 300);
    assert!(!s.conversation_progress().unwrap().ready);
    indexed(&s);
    let mut ctx = context();
    let result = s.search_conversations(&query(), &ctx).unwrap();
    assert_eq!(result.rows[0].matching_count, 501);
    ctx.generation = Some(result.generation);
    assert!(s
        .preview_conversation(
            &query(),
            &ctx,
            &result.rows[0].conversation_id,
            true,
            ConversationAction::Archive
        )
        .is_err());
    let mut q = query();
    q.offset = 1;
    assert!(s.search_conversations(&q, &context()).is_err());
    assert!(s.search_conversations(&q, &ctx).unwrap().rows.is_empty());
}

#[test]
fn encrypted_restart_resumes_staged_headers_and_preserves_preview_scope() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("local.db");
    let key = DatabaseKey::from_bytes([93; 32]);
    let token;
    {
        let s = SqliteMailStore::open(&path, &key, Default::default()).unwrap();
        s.transaction(|tx| {
            tx.execute_batch("INSERT INTO accounts VALUES ('a','a@example.com','synthetic',0,0)")
                .map_err(error)?;
            Ok(())
        })
        .unwrap();
        for i in 0..301 {
            insert(
                &s,
                &format!("m{i:04}"),
                "a",
                i + 1,
                None,
                Some("root@example.org"),
                "INBOX",
            );
        }
        assert_eq!(s.backfill_conversations().unwrap(), 300);
        assert!(!s.conversation_progress().unwrap().ready);
    }
    {
        let s = SqliteMailStore::open(&path, &key, Default::default()).unwrap();
        indexed(&s);
        let page = s.search_conversations(&query(), &context()).unwrap();
        assert_eq!(page.rows[0].total_count, 301);
        let mut ctx = context();
        ctx.generation = Some(page.generation);
        let preview = s
            .preview_conversation(
                &query(),
                &ctx,
                &page.rows[0].conversation_id,
                false,
                ConversationAction::Tag("local".into()),
            )
            .unwrap();
        token = preview.token;
    }
    let s = SqliteMailStore::open(&path, &key, Default::default()).unwrap();
    let (generation, ids, _) = s.conversation_preview(&token).unwrap();
    let forged = vec![(
        ids[0].clone(),
        crate::desktop::LocalMutation::Read(true),
        None,
    )];
    assert!(s
        .apply_preview_mutations(&forged, &generation, &token)
        .is_err());
    assert!(s.conversation_preview(&token).is_ok());
    let changes: Vec<_> = ids
        .iter()
        .map(|id| {
            (
                id.clone(),
                crate::desktop::LocalMutation::Tag("local"),
                None,
            )
        })
        .collect();
    s.apply_preview_mutations(&changes, &generation, &token)
        .unwrap();
    assert!(s.conversation_preview(&token).is_err());
    assert!(SqliteMailStore::open(
        &path,
        &DatabaseKey::from_bytes([94; 32]),
        Default::default()
    )
    .is_err());
}
#[test]
fn saved_category_review_and_semantic_membership_precede_group_pagination() {
    use crate::saved_views::{SavedView, ViewFilters};
    use crate::sender_review::{ReviewState, SenderChoice};
    let s = fixture();
    insert(&s, "root", "a", 1, Some("root@example.org"), None, "INBOX");
    insert(
        &s,
        "reply",
        "a",
        2,
        None,
        Some("root@example.org"),
        "Archive",
    );
    insert(&s, "other", "a", 3, None, None, "INBOX");
    s.transaction(|tx| {
        tx.execute(
            "UPDATE messages SET is_starred=1,category_badge='Financial' WHERE id='root'",
            [],
        )
        .map_err(error)?;
        Ok(())
    })
    .unwrap();
    for (id, axis) in [("root", 0), ("reply", 1), ("other", 2)] {
        let mut vector = vec![0; 384];
        vector[axis] = 100;
        s.insert_message_embedding(id, &vector).unwrap();
    }
    indexed(&s);
    let mut q = query();
    q.limit = 1;
    let mut ctx = context();
    ctx.source = ConversationSource::Category(triage_ai::TriageCategory::Financial);
    let result = s.search_conversations(&q, &ctx).unwrap();
    assert_eq!(result.rows[0].representative.id, "root");
    assert_eq!(result.rows[0].matching_count, 1);
    assert_eq!(result.rows[0].total_count, 2);
    let saved = s
        .save_saved_view(SavedView {
            version: 1,
            id: "starred".into(),
            revision: 0,
            name: "Starred".into(),
            query: "".into(),
            mode: SearchMode::Lexical,
            accounts: vec!["a".into()],
            scope: None,
            filters: ViewFilters {
                is_starred: Some(true),
                ..Default::default()
            },
        })
        .unwrap();
    ctx.source = ConversationSource::Saved {
        id: saved.id,
        revision: saved.revision,
    };
    assert_eq!(
        s.search_conversations(&q, &ctx).unwrap().rows[0].matching_count,
        1
    );
    let mut prefs = s.review_preferences("a").unwrap();
    prefs.enabled = true;
    prefs.hide_in_inbox = true;
    s.save_review_preferences(prefs).unwrap();
    s.backfill_sender_review().unwrap();
    let choice = s.list_sender_choices("a", 0).unwrap().remove(0);
    s.save_sender_choice(SenderChoice {
        state: ReviewState::Pending,
        ..choice
    })
    .unwrap();
    ctx.source = ConversationSource::Review(ReviewState::Pending);
    assert_eq!(
        s.search_conversations(&q, &ctx).unwrap().rows[0]
            .representative
            .id,
        "other"
    );
    ctx.source = ConversationSource::Messages;
    ctx.scope = Some(MailboxScope::AccountView {
        account_id: "a".into(),
        view: MailboxView::Inbox,
    });
    assert!(s.search_conversations(&q, &ctx).unwrap().rows.is_empty());
    ctx.scope = None;
    q.mode = SearchMode::Semantic;
    q.query_embedding_int8 = Some({
        let mut v = vec![0; 384];
        v[1] = 100;
        v
    });
    let first = s.search_conversations(&q, &ctx).unwrap();
    assert_eq!(first.rows[0].representative.id, "reply");
    assert_eq!(first.rows[0].matching_count, 2);
    q.offset = 1;
    ctx.generation = Some(first.generation);
    let second = s.search_conversations(&q, &ctx).unwrap();
    assert_eq!(second.rows[0].representative.id, "other");
}

#[test]
fn ready_account_remains_usable_while_another_account_is_initializing() {
    let s = fixture();
    insert(&s, "root", "a", 1, None, None, "INBOX");
    indexed(&s);
    insert(&s, "other", "b", 1, None, None, "INBOX");
    assert!(s.conversation_progress_for(Some("a")).unwrap().ready);
    assert!(!s.conversation_progress().unwrap().ready);
    assert!(s.search_conversations(&query(), &context()).is_err());
    let mut ctx = context();
    ctx.scope = Some(MailboxScope::AccountView {
        account_id: "a".into(),
        view: MailboxView::Inbox,
    });
    let page = s.search_conversations(&query(), &ctx).unwrap();
    ctx.generation = Some(page.generation);
    let preview = s
        .preview_conversation(
            &query(),
            &ctx,
            &page.rows[0].conversation_id,
            false,
            ConversationAction::Tag("ready".into()),
        )
        .unwrap();
    let (generation, ids, _) = s.conversation_preview(&preview.token).unwrap();
    s.apply_preview_mutations(
        &[(
            ids[0].clone(),
            crate::desktop::LocalMutation::Tag("ready"),
            None,
        )],
        &generation,
        &preview.token,
    )
    .unwrap();
}
