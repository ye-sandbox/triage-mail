CREATE TABLE alert_destinations (
 id TEXT PRIMARY KEY NOT NULL, revision INTEGER NOT NULL, definition_json TEXT NOT NULL,
 secret_key TEXT, next_start INTEGER NOT NULL DEFAULT 0
);
CREATE TABLE alert_deleted_ids (id TEXT PRIMARY KEY NOT NULL);
CREATE TABLE alert_secret_intents (secret_key TEXT PRIMARY KEY NOT NULL);
CREATE TABLE alert_dedup (
 account_id TEXT NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
 destination_id TEXT NOT NULL REFERENCES alert_destinations(id) ON DELETE CASCADE,
 event_key TEXT NOT NULL, PRIMARY KEY(account_id,destination_id,event_key)
);
CREATE TABLE alert_jobs (
 id TEXT PRIMARY KEY NOT NULL,
 destination_id TEXT NOT NULL REFERENCES alert_destinations(id) ON DELETE CASCADE,
 revision INTEGER NOT NULL,
 account_id TEXT REFERENCES accounts(id) ON DELETE CASCADE,
 message_id TEXT REFERENCES messages(id) ON DELETE CASCADE,
 payload TEXT NOT NULL,
 created INTEGER NOT NULL, due INTEGER NOT NULL, lease_until INTEGER NOT NULL DEFAULT 0,
 attempts INTEGER NOT NULL DEFAULT 0, status TEXT NOT NULL DEFAULT 'Pending',
 finished INTEGER
);
CREATE INDEX idx_alert_jobs_due ON alert_jobs(status,due,lease_until);
ALTER TABLE rule_jobs ADD COLUMN alert_revisions TEXT NOT NULL DEFAULT '{}';
ALTER TABLE rule_jobs ADD COLUMN event_key TEXT NOT NULL DEFAULT '';
