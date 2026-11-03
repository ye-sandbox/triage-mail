CREATE TABLE alert_schedule_policies (
 destination_id TEXT PRIMARY KEY REFERENCES alert_destinations(id) ON DELETE CASCADE,
 revision INTEGER NOT NULL, definition_json TEXT NOT NULL,
 last_slot INTEGER, summarized INTEGER NOT NULL DEFAULT 0,
 expired INTEGER NOT NULL DEFAULT 0, overflow INTEGER NOT NULL DEFAULT 0
);
CREATE TABLE alert_schedule_items (
 id TEXT PRIMARY KEY, destination_id TEXT NOT NULL REFERENCES alert_destinations(id) ON DELETE CASCADE,
 destination_revision INTEGER NOT NULL, policy_revision INTEGER NOT NULL,
 account_id TEXT NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
 message_id TEXT NOT NULL REFERENCES messages(id) ON DELETE CASCADE,
 reminder_id TEXT REFERENCES reminder_jobs(id) ON DELETE CASCADE,
 payload TEXT NOT NULL, created INTEGER NOT NULL,
 sealed_job TEXT REFERENCES alert_jobs(id) ON DELETE CASCADE
);
CREATE INDEX alert_schedule_items_pending ON alert_schedule_items(destination_id,sealed_job,created);
ALTER TABLE alert_jobs ADD COLUMN schedule_revision INTEGER;
ALTER TABLE alert_jobs ADD COLUMN urgent INTEGER NOT NULL DEFAULT 0;
ALTER TABLE rule_jobs ADD COLUMN schedule_revisions TEXT NOT NULL DEFAULT '{}';
-- INVARIANT: Existing automatic jobs must participate in policy edits/quiet gating;
-- explicit tests/replays retain their immediate bypass. No old payload is re-enqueued.
UPDATE alert_jobs SET schedule_revision=0 WHERE id LIKE 'auto-%' OR id LIKE 'rem-%';
-- INVARIANT: Removing a constituent invalidates the entire unsent immutable summary.
CREATE TRIGGER alert_schedule_remove BEFORE DELETE ON alert_schedule_items
WHEN OLD.sealed_job IS NOT NULL BEGIN
 UPDATE alert_jobs SET status='Canceled',payload='',lease_until=0,finished=created
 WHERE id=OLD.sealed_job AND status='Pending';
END;
CREATE TRIGGER alert_schedule_terminal AFTER UPDATE OF status ON alert_jobs
WHEN NEW.status!='Pending' BEGIN
 UPDATE alert_schedule_policies SET expired=expired+(SELECT count(*) FROM alert_schedule_items WHERE sealed_job=NEW.id)
 WHERE destination_id=NEW.destination_id AND NEW.status='Expired';
 DELETE FROM alert_schedule_items WHERE sealed_job=NEW.id;
END;
CREATE TRIGGER alert_schedule_destination AFTER UPDATE OF revision ON alert_destinations BEGIN
 DELETE FROM alert_schedule_items WHERE destination_id=NEW.id;
END;
CREATE TRIGGER alert_schedule_read AFTER UPDATE OF is_read ON messages WHEN NEW.is_read=1 BEGIN
 DELETE FROM alert_schedule_items WHERE message_id=NEW.id AND reminder_id IN
 (SELECT id FROM reminder_jobs WHERE trigger_type='UnreadInterval');
 UPDATE alert_jobs SET status='Canceled',payload='',lease_until=0,finished=created
 WHERE message_id=NEW.id AND id IN
 (SELECT 'rem-' || j.id || '-' || d.id FROM reminder_jobs j CROSS JOIN alert_destinations d
 WHERE j.message_id=NEW.id AND j.trigger_type='UnreadInterval') AND status='Pending';
END;
CREATE TRIGGER alert_schedule_dismiss AFTER INSERT ON due_date_dismissals BEGIN
 DELETE FROM alert_schedule_items WHERE message_id=NEW.message_id AND reminder_id IS NOT NULL;
 UPDATE alert_jobs SET status='Canceled',payload='',lease_until=0,finished=created
 WHERE message_id=NEW.message_id AND id LIKE 'rem-%' AND status='Pending';
END;

CREATE TABLE alert_schedule_cursor (singleton INTEGER PRIMARY KEY CHECK(singleton=1), destination_id TEXT NOT NULL);
INSERT INTO alert_schedule_cursor VALUES (1,'');
CREATE TRIGGER alert_schedule_config_delete AFTER DELETE ON reminder_configs BEGIN
 DELETE FROM alert_schedule_items WHERE reminder_id IN (SELECT id FROM reminder_jobs WHERE config_id=OLD.id);
 UPDATE alert_jobs SET status='Canceled',payload='',lease_until=0,finished=created
 WHERE status='Pending' AND id IN (SELECT 'rem-' || j.id || '-' || d.id FROM reminder_jobs j CROSS JOIN alert_destinations d WHERE j.config_id=OLD.id);
END;
CREATE TRIGGER alert_schedule_config_edit AFTER UPDATE OF revision ON reminder_configs BEGIN
 UPDATE reminder_jobs SET status='Canceled' WHERE config_id=NEW.id AND status='Pending';
 DELETE FROM alert_schedule_items WHERE reminder_id IN (SELECT id FROM reminder_jobs WHERE config_id=NEW.id);
 UPDATE alert_jobs SET status='Canceled',payload='',lease_until=0,finished=created
 WHERE status='Pending' AND id IN (SELECT 'rem-' || j.id || '-' || d.id FROM reminder_jobs j CROSS JOIN alert_destinations d WHERE j.config_id=NEW.id);
END;
