use triage_ai::rules::*;

fn rule(id: &str) -> Rule {
    Rule {
        version: 1,
        id: id.into(),
        revision: 0,
        name: "Synthetic rule".into(),
        enabled: true,
        position: 0,
        all: vec![Condition::Account("a".into())],
        any: vec![],
        except: vec![],
        actions: vec![RuleAction::Star],
        stop_processing: false,
    }
}
fn facts() -> MessageFacts {
    MessageFacts {
        account_id: "a".into(),
        sender_address: sender_address("Billing <billing@sub.example.com>"),
        recipients: vec![],
        subject: "Monthly BILL".into(),
        body: Some("Amount attached".into()),
        attachments: vec![("Invoice.PDF".into(), "application/pdf".into())],
        amounts_cents: vec![12500],
        due_dates: vec!["2028-02-29".into()],
    }
}

#[test]
fn compound_groups_exceptions_and_missing_facts_are_explained() {
    let mut r = rule("r");
    r.all.extend([
        Condition::DomainSuffix("example.com".into()),
        Condition::AttachmentMime("PDF".into()),
        Condition::AmountAtLeast(12500),
        Condition::DueOnOrBefore("2028-02-29".into()),
    ]);
    r.any = vec![
        Condition::Subject("bill".into()),
        Condition::Body("absent".into()),
    ];
    let f = facts();
    assert_eq!(evaluate(&[r.clone()], &f).actions, vec![RuleAction::Star]);
    r.except = vec![Condition::Body("attached".into())];
    let p = evaluate(&[r.clone()], &f);
    assert!(p.actions.is_empty());
    assert_eq!(p.explanations[0].outcome, RuleOutcome::Excluded);
    r.except.clear();
    r.all.push(Condition::Body("anything".into()));
    let mut f = f;
    f.body = None;
    let p = evaluate(&[r], &f);
    assert!(p.actions.is_empty());
    assert_eq!(
        p.explanations[0].all.last(),
        Some(&ConditionResult::MissingFact)
    );
}

#[test]
fn ordering_ties_stops_and_additive_conflicts_are_deterministic() {
    let mut a = rule("a");
    a.actions = vec![
        RuleAction::Archive,
        RuleAction::Tag("bill".into()),
        RuleAction::Star,
    ];
    let mut b = rule("b");
    b.actions = vec![RuleAction::Star, RuleAction::Tag("bill".into())];
    b.stop_processing = true;
    let c = rule("c");
    let p = evaluate(&[c, b, a], &facts());
    assert_eq!(
        p.actions,
        vec![
            RuleAction::Tag("bill".into()),
            RuleAction::Star,
            RuleAction::Archive
        ]
    );
    assert_eq!(
        p.explanations
            .iter()
            .map(|e| e.rule_id.as_str())
            .collect::<Vec<_>>(),
        ["a", "b", "c"]
    );
    assert_eq!(p.explanations[2].outcome, RuleOutcome::Stopped);
}

#[test]
fn address_parser_domain_boundaries_and_exact_accounts_fail_closed() {
    assert_eq!(
        sender_address("Name <USER@Example.COM>"),
        Some("user@example.com".into())
    );
    assert_eq!(sender_address("a@example.com, b@example.org"), None);
    assert_eq!(sender_address("a@example.com\r\nBcc: b@example.org"), None);
    assert_eq!(sender_address("not an address"), None);
    let mut r = rule("r");
    r.all = vec![Condition::Domain("example.com".into())];
    assert!(evaluate(&[r.clone()], &facts()).actions.is_empty());
    r.all = vec![Condition::DomainSuffix("example.test".into())];
    let mut f = facts();
    f.sender_address = Some("a@evil-example.test".into());
    assert!(evaluate(&[r], &f).actions.is_empty());
    f.account_id = "A".into();
    assert!(evaluate(&[rule("a")], &f).actions.is_empty());
}

#[test]
fn amounts_dates_and_definitions_are_bounded_and_exact() {
    for date in ["2028-02-29", "2000-02-29", "2026-12-31"] {
        assert!(valid_date(date));
    }
    for date in [
        "2026-02-29",
        "1900-02-29",
        "0000-01-01",
        "2026-00-01",
        "2026-13-01",
        "é026-01-01",
    ] {
        assert!(!valid_date(date));
    }
    let mut r = rule("r");
    r.all = vec![Condition::AmountAtMost(12499)];
    assert!(evaluate(&[r.clone()], &facts()).actions.is_empty());
    r.all = vec![Condition::AmountAtLeast(12500)];
    assert_eq!(evaluate(&[r.clone()], &facts()).actions.len(), 1);
    r.all.clear();
    assert!(!r.validate());
    r.any = vec![Condition::Subject("".into())];
    assert!(!r.validate());
    r.any = vec![Condition::DueOnOrAfter("2026-02-29".into())];
    assert!(!r.validate());
    r.any = vec![Condition::Subject("bill".into()); 65];
    assert!(!r.validate());
}

#[test]
fn recipient_conditions_match_forwarded_alias_and_report_missing_facts() {
    let mut r = rule("rcpt");
    r.all = vec![
        Condition::Recipient("Alias@example.org".into()),
        Condition::RecipientDomain("example.org".into()),
    ];
    let mut f = facts();
    assert_eq!(
        evaluate(&[r.clone()], &f).explanations[0].all,
        vec![ConditionResult::MissingFact; 2]
    );
    f.recipients = triage_ai::rules::recipient_addresses("Me <me@example.com>, alias@example.org");
    assert_eq!(f.recipients, vec!["me@example.com", "alias@example.org"]);
    assert_eq!(evaluate(&[r.clone()], &f).actions, vec![RuleAction::Star]);
    r.all = vec![Condition::Recipient("other@example.org".into())];
    assert!(evaluate(&[r], &f).actions.is_empty());
    assert!(triage_ai::rules::recipient_addresses("a@b\r\nc").is_empty());
}
