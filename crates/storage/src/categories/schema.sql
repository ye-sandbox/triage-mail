CREATE TABLE message_categories (
    message_id TEXT PRIMARY KEY REFERENCES messages(id) ON DELETE CASCADE,
    account_id TEXT NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    sender_key TEXT,
    correction_key TEXT NOT NULL,
    durable INTEGER NOT NULL,
    automatic TEXT NOT NULL,
    reason TEXT NOT NULL,
    version INTEGER NOT NULL
);
CREATE INDEX idx_categories_account_sender ON message_categories(account_id,sender_key);
CREATE INDEX idx_categories_version ON message_categories(version,message_id);
CREATE TABLE category_corrections (
    account_id TEXT NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    correction_key TEXT NOT NULL,
    revision INTEGER NOT NULL,
    category TEXT,
    PRIMARY KEY(account_id,correction_key)
);
CREATE TABLE category_sender_overrides (
    account_id TEXT NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    sender_key TEXT NOT NULL,
    revision INTEGER NOT NULL,
    category TEXT,
    PRIMARY KEY(account_id,sender_key)
);
-- INVARIANT: Record-only corrections cannot be inherited by a later reused canonical ID.
CREATE TRIGGER category_record_correction_delete BEFORE DELETE ON messages BEGIN
    DELETE FROM category_corrections WHERE account_id=old.account_id AND correction_key IN
      (SELECT correction_key FROM message_categories WHERE message_id=old.id AND durable=0);
END;
-- INVARIANT: Any stored fact change invalidates only the automatic classification.
CREATE TRIGGER category_facts_changed AFTER UPDATE OF subject,sender,text_body,triage_badges,one_click_unsubscribe_supported ON messages BEGIN
    UPDATE message_categories SET version=0 WHERE message_id=new.id;
END;
