---
id: "k-028"
title: "Saved-view eligibility and deferred editor bindings"
date: 2026-11-02
tags: [sqlite, search, svelte, lifecycle]
severity: medium
affected_components: [crates/storage, apps/desktop]
---

# Saved-view eligibility and deferred editor bindings

- Reuse private ScopedSearch, substituting general eligibility before lexical LIMIT and
  exact vector ranking, including both hybrid candidate windows. Do not globally expand
  mailbox account predicates to `account='' OR account_id=?`; that can lose indexed plans.
  Existing mailbox SQL must still contain the direct account equality.
- SQL function closures capture parsed-filter definitions for one mutex-protected search.
  Register and remove inside the transaction boundary, removing on execution errors too.
  A cached statement must not retain sensitive filter snapshots between requests. Use
  parameter-bound JSON, never interpolate account/sender/tag strings into SQL.
- Missing account references must remain explicit and invalid; pruning them can turn an
  emptied selection into All Mail. Listing and revision-safe deletion still work.
- A nullable parent draft passed through a binding can become null during teardown before
  Svelte's deferred input default-value callback executes. Keep the editor's initial object
  reference stable with untrack; share nested reactive fields without binding the nullable
  parent reference. Test completion after unmount and check for unhandled test rejections.
- Upcoming-bills windows use UTC date ordinals; all date predicates apply to one valid
  boleto date. Re-arm midnight refresh after authoritative updates and refresh on focus.
