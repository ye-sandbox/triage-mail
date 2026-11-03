CREATE TABLE sender_review_preferences (
    account_id TEXT PRIMARY KEY REFERENCES accounts(id) ON DELETE CASCADE,
    revision INTEGER NOT NULL, enabled INTEGER NOT NULL,
    hide_in_inbox INTEGER NOT NULL, initializing INTEGER NOT NULL
);
CREATE TABLE sender_review_choices (
    account_id TEXT NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    sender TEXT NOT NULL, revision INTEGER NOT NULL, state TEXT NOT NULL
        CHECK(state IN ('Known','Pending','Accepted','Declined')),
    PRIMARY KEY(account_id,sender)
);
CREATE TABLE sender_review_messages (
    message_id TEXT PRIMARY KEY REFERENCES messages(id) ON DELETE CASCADE,
    account_id TEXT NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    sender TEXT
);
CREATE INDEX idx_review_sender ON sender_review_messages(account_id,sender,message_id);
CREATE TABLE sender_review_frontiers (
    account_id TEXT NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    folder TEXT NOT NULL, uidvalidity INTEGER NOT NULL, frontier INTEGER NOT NULL,
    PRIMARY KEY(account_id,folder)
);
