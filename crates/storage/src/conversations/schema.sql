CREATE TABLE conversation_epoch (id INTEGER PRIMARY KEY CHECK(id=1), generation TEXT NOT NULL);
INSERT INTO conversation_epoch VALUES (1,lower(hex(randomblob(16))));
CREATE TABLE conversation_dirty (account_id TEXT PRIMARY KEY REFERENCES accounts(id) ON DELETE CASCADE);
INSERT INTO conversation_dirty SELECT id FROM accounts;
CREATE TABLE conversation_inputs (
 message_id TEXT PRIMARY KEY REFERENCES messages(id) ON DELETE CASCADE,
 account_id TEXT NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
 header_id TEXT, parent_id TEXT, refs TEXT NOT NULL
);
CREATE INDEX idx_conversation_inputs_account ON conversation_inputs(account_id,message_id);
CREATE TABLE conversation_members (
 message_id TEXT PRIMARY KEY REFERENCES messages(id) ON DELETE CASCADE,
 account_id TEXT NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
 conversation_id TEXT NOT NULL
);
CREATE INDEX idx_conversation_members_group ON conversation_members(conversation_id,message_id);
CREATE TABLE conversation_previews (
 token TEXT PRIMARY KEY, generation TEXT NOT NULL, ids TEXT NOT NULL, action TEXT NOT NULL,
 created_at INTEGER NOT NULL
);
CREATE TRIGGER conversation_insert AFTER INSERT ON messages BEGIN
 INSERT OR IGNORE INTO conversation_dirty VALUES (new.account_id);
 UPDATE conversation_epoch SET generation=lower(hex(randomblob(16)));
END;
CREATE TRIGGER conversation_headers AFTER UPDATE OF account_id,message_id_header,in_reply_to,references_header ON messages
WHEN old.account_id IS NOT new.account_id OR old.message_id_header IS NOT new.message_id_header
 OR old.in_reply_to IS NOT new.in_reply_to OR old.references_header IS NOT new.references_header BEGIN
 DELETE FROM conversation_inputs WHERE message_id=new.id;
 INSERT OR IGNORE INTO conversation_dirty VALUES (old.account_id),(new.account_id);
END;
CREATE TRIGGER conversation_delete BEFORE DELETE ON messages BEGIN
 INSERT OR IGNORE INTO conversation_dirty SELECT id FROM accounts WHERE id=old.account_id;
 UPDATE conversation_epoch SET generation=lower(hex(randomblob(16)));
END;
CREATE TRIGGER conversation_update AFTER UPDATE ON messages BEGIN
 UPDATE conversation_epoch SET generation=lower(hex(randomblob(16)));
END;
CREATE TRIGGER conversation_epoch_accounts_insert AFTER INSERT ON accounts BEGIN UPDATE conversation_epoch SET generation=lower(hex(randomblob(16))); END;
CREATE TRIGGER conversation_epoch_accounts_update AFTER UPDATE ON accounts BEGIN UPDATE conversation_epoch SET generation=lower(hex(randomblob(16))); END;
CREATE TRIGGER conversation_epoch_accounts_delete AFTER DELETE ON accounts BEGIN UPDATE conversation_epoch SET generation=lower(hex(randomblob(16))); DELETE FROM conversation_previews; END;
CREATE TRIGGER conversation_epoch_tags_insert AFTER INSERT ON tags BEGIN UPDATE conversation_epoch SET generation=lower(hex(randomblob(16))); END;
CREATE TRIGGER conversation_epoch_tags_update AFTER UPDATE ON tags BEGIN UPDATE conversation_epoch SET generation=lower(hex(randomblob(16))); END;
CREATE TRIGGER conversation_epoch_tags_delete AFTER DELETE ON tags BEGIN UPDATE conversation_epoch SET generation=lower(hex(randomblob(16))); END;
CREATE TRIGGER conversation_epoch_message_folder_overrides_insert AFTER INSERT ON message_folder_overrides BEGIN UPDATE conversation_epoch SET generation=lower(hex(randomblob(16))); END;
CREATE TRIGGER conversation_epoch_message_folder_overrides_update AFTER UPDATE ON message_folder_overrides BEGIN UPDATE conversation_epoch SET generation=lower(hex(randomblob(16))); END;
CREATE TRIGGER conversation_epoch_message_folder_overrides_delete AFTER DELETE ON message_folder_overrides BEGIN UPDATE conversation_epoch SET generation=lower(hex(randomblob(16))); END;
CREATE TRIGGER conversation_epoch_message_folder_memberships_insert AFTER INSERT ON message_folder_memberships BEGIN UPDATE conversation_epoch SET generation=lower(hex(randomblob(16))); END;
CREATE TRIGGER conversation_epoch_message_folder_memberships_update AFTER UPDATE ON message_folder_memberships BEGIN UPDATE conversation_epoch SET generation=lower(hex(randomblob(16))); END;
CREATE TRIGGER conversation_epoch_message_folder_memberships_delete AFTER DELETE ON message_folder_memberships BEGIN UPDATE conversation_epoch SET generation=lower(hex(randomblob(16))); END;
CREATE TRIGGER conversation_epoch_folders_insert AFTER INSERT ON folders BEGIN UPDATE conversation_epoch SET generation=lower(hex(randomblob(16))); END;
CREATE TRIGGER conversation_epoch_folders_update AFTER UPDATE ON folders BEGIN UPDATE conversation_epoch SET generation=lower(hex(randomblob(16))); END;
CREATE TRIGGER conversation_epoch_folders_delete AFTER DELETE ON folders BEGIN UPDATE conversation_epoch SET generation=lower(hex(randomblob(16))); END;
CREATE TRIGGER conversation_epoch_saved_views_insert AFTER INSERT ON saved_views BEGIN UPDATE conversation_epoch SET generation=lower(hex(randomblob(16))); END;
CREATE TRIGGER conversation_epoch_saved_views_update AFTER UPDATE ON saved_views BEGIN UPDATE conversation_epoch SET generation=lower(hex(randomblob(16))); END;
CREATE TRIGGER conversation_epoch_saved_views_delete AFTER DELETE ON saved_views BEGIN UPDATE conversation_epoch SET generation=lower(hex(randomblob(16))); END;
CREATE TRIGGER conversation_epoch_message_categories_insert AFTER INSERT ON message_categories BEGIN UPDATE conversation_epoch SET generation=lower(hex(randomblob(16))); END;
CREATE TRIGGER conversation_epoch_message_categories_update AFTER UPDATE ON message_categories BEGIN UPDATE conversation_epoch SET generation=lower(hex(randomblob(16))); END;
CREATE TRIGGER conversation_epoch_message_categories_delete AFTER DELETE ON message_categories BEGIN UPDATE conversation_epoch SET generation=lower(hex(randomblob(16))); END;
CREATE TRIGGER conversation_epoch_category_corrections_insert AFTER INSERT ON category_corrections BEGIN UPDATE conversation_epoch SET generation=lower(hex(randomblob(16))); END;
CREATE TRIGGER conversation_epoch_category_corrections_update AFTER UPDATE ON category_corrections BEGIN UPDATE conversation_epoch SET generation=lower(hex(randomblob(16))); END;
CREATE TRIGGER conversation_epoch_category_corrections_delete AFTER DELETE ON category_corrections BEGIN UPDATE conversation_epoch SET generation=lower(hex(randomblob(16))); END;
CREATE TRIGGER conversation_epoch_category_sender_overrides_insert AFTER INSERT ON category_sender_overrides BEGIN UPDATE conversation_epoch SET generation=lower(hex(randomblob(16))); END;
CREATE TRIGGER conversation_epoch_category_sender_overrides_update AFTER UPDATE ON category_sender_overrides BEGIN UPDATE conversation_epoch SET generation=lower(hex(randomblob(16))); END;
CREATE TRIGGER conversation_epoch_category_sender_overrides_delete AFTER DELETE ON category_sender_overrides BEGIN UPDATE conversation_epoch SET generation=lower(hex(randomblob(16))); END;
CREATE TRIGGER conversation_epoch_sender_review_preferences_insert AFTER INSERT ON sender_review_preferences BEGIN UPDATE conversation_epoch SET generation=lower(hex(randomblob(16))); END;
CREATE TRIGGER conversation_epoch_sender_review_preferences_update AFTER UPDATE ON sender_review_preferences BEGIN UPDATE conversation_epoch SET generation=lower(hex(randomblob(16))); END;
CREATE TRIGGER conversation_epoch_sender_review_preferences_delete AFTER DELETE ON sender_review_preferences BEGIN UPDATE conversation_epoch SET generation=lower(hex(randomblob(16))); END;
CREATE TRIGGER conversation_epoch_sender_review_choices_insert AFTER INSERT ON sender_review_choices BEGIN UPDATE conversation_epoch SET generation=lower(hex(randomblob(16))); END;
CREATE TRIGGER conversation_epoch_sender_review_choices_update AFTER UPDATE ON sender_review_choices BEGIN UPDATE conversation_epoch SET generation=lower(hex(randomblob(16))); END;
CREATE TRIGGER conversation_epoch_sender_review_choices_delete AFTER DELETE ON sender_review_choices BEGIN UPDATE conversation_epoch SET generation=lower(hex(randomblob(16))); END;
CREATE TRIGGER conversation_epoch_sender_review_messages_insert AFTER INSERT ON sender_review_messages BEGIN UPDATE conversation_epoch SET generation=lower(hex(randomblob(16))); END;
CREATE TRIGGER conversation_epoch_sender_review_messages_update AFTER UPDATE ON sender_review_messages BEGIN UPDATE conversation_epoch SET generation=lower(hex(randomblob(16))); END;
CREATE TRIGGER conversation_epoch_sender_review_messages_delete AFTER DELETE ON sender_review_messages BEGIN UPDATE conversation_epoch SET generation=lower(hex(randomblob(16))); END;
CREATE TABLE conversation_attempts (
 message_id TEXT PRIMARY KEY REFERENCES messages(id) ON DELETE CASCADE,
 failed INTEGER NOT NULL, attempts INTEGER NOT NULL
);
