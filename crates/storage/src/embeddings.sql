CREATE TABLE embedding_generation (
    singleton INTEGER PRIMARY KEY CHECK(singleton=1),
    generation TEXT NOT NULL,
    epoch TEXT NOT NULL
);
CREATE TABLE embedding_work (
    message_id TEXT PRIMARY KEY REFERENCES messages(id) ON DELETE CASCADE,
    version TEXT NOT NULL,
    indexed_epoch TEXT,
    failed_epoch TEXT
);
CREATE INDEX embedding_pending ON embedding_work(indexed_epoch,failed_epoch,message_id);
INSERT INTO embedding_work(message_id,version) SELECT id,lower(hex(randomblob(16))) FROM messages;
CREATE TRIGGER embedding_insert AFTER INSERT ON messages BEGIN
    INSERT OR REPLACE INTO embedding_work(message_id,version) VALUES(new.id,lower(hex(randomblob(16))));
END;
CREATE TRIGGER embedding_update AFTER UPDATE OF subject,text_body ON messages
WHEN old.subject IS NOT new.subject OR old.text_body IS NOT new.text_body BEGIN
    UPDATE embedding_work SET version=lower(hex(randomblob(16))),indexed_epoch=NULL,failed_epoch=NULL WHERE message_id=new.id;
    DELETE FROM vec_messages WHERE message_id=new.id AND EXISTS(SELECT 1 FROM embedding_generation);
END;
CREATE TRIGGER embedding_delete BEFORE DELETE ON messages BEGIN
    DELETE FROM vec_messages WHERE message_id=old.id AND EXISTS(SELECT 1 FROM embedding_generation);
END;
