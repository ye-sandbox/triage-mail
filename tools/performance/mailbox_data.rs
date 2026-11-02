//! Shared disposable mailbox data for storage and native validation only.
use std::{error::Error, io::Read};
use storage::{
    connection::open_encrypted_connection, migrations::SchemaMigrator, DatabaseKey,
    FtsTokenizerMode,
};

pub const ROWS: usize = 100_000;
pub const DIMENSIONS: usize = 384;
pub type Result<T> = std::result::Result<T, Box<dyn Error>>;
pub struct Fixture {
    pub directory: tempfile::TempDir,
    pub key: DatabaseKey,
}
pub fn vector(id: usize) -> Vec<i8> {
    let mut state = id as u64 + 1;
    (0..DIMENSIONS)
        .map(|_| {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            (state % 255) as i8
        })
        .collect()
}
pub fn folder(id: usize) -> &'static str {
    if (id / 4).is_multiple_of(5) {
        "Archive"
    } else {
        "INBOX"
    }
}
pub fn active_snooze(id: usize) -> bool {
    id / 4 % 10 == 1
}
pub fn starred(id: usize) -> bool {
    id / 4 < 31
}

pub fn seed() -> Result<Fixture> {
    let directory = tempfile::tempdir()?;
    // INVARIANT: The runtime key is OS-random, never logged or written to disk.
    let mut bytes = [0; 32];
    std::fs::File::open("/dev/urandom")?.read_exact(&mut bytes)?;
    let key = DatabaseKey::from_bytes(bytes);
    let path = directory.path().join("synthetic.db");
    let mut conn = open_encrypted_connection(&path, &key)?;
    SchemaMigrator::new(FtsTokenizerMode::default()).migrate(&mut conn)?;
    let tx = conn.transaction()?;
    for account in 0..4 {
        let account = format!("account-{account}");
        tx.execute(
            "INSERT INTO accounts VALUES (?1,?2,'synthetic',0,0)",
            (&account, format!("{account}@example.com")),
        )?;
        for (name, role) in [
            ("INBOX", "inbox"),
            ("Archive", "archive"),
            ("Empty", "other"),
            ("All Mail", "other"),
        ] {
            tx.execute("INSERT INTO folders(account_id,folder_name,display_name,role,updated_at_epoch_ms) VALUES (?1,?2,?2,?3,0)", (&account,name,role))?;
        }
    }
    {
        let mut message = tx.prepare("INSERT INTO messages(id,account_id,folder,uid,subject,sender,recipients,received_at_epoch_ms,raw_headers,text_body,created_at_epoch_ms,is_starred,snooze_until_epoch_ms,provider_message_id) VALUES (?1,?2,?3,?4,'Synthetic newsletter','sender@example.org','reader@example.com',?4,'','newsletter receipt shipment',0,?5,?6,?1)")?;
        let mut embedding = tx.prepare("INSERT INTO vec_messages(message_id,account_id,embedding_int8) VALUES (?1,?2,vec_int8(?3))")?;
        let mut member = tx.prepare("INSERT INTO message_folder_memberships(account_id,folder,uid,message_id,created_at_epoch_ms) VALUES (?1,?2,?3,?4,0)")?;
        let now = storage::now_epoch_ms();
        for id in 0..ROWS {
            let name = format!("message-{id:06}");
            let account = format!("account-{}", id % 4);
            let snooze = if active_snooze(id) {
                Some(now + 86_400_000)
            } else if id / 4 % 10 == 2 {
                Some(now - 86_400_000)
            } else {
                None
            };
            message.execute((&name, &account, folder(id), id + 1, starred(id), snooze))?;
            let bytes: Vec<u8> = vector(id).iter().map(|&v| v as u8).collect();
            embedding.execute((&name, &account, &bytes))?;
            member.execute((&account, folder(id), id + 1, &name))?;
            // INVARIANT: Duplicate label memberships retain one body/vector and canonical UID.
            member.execute((&account, "All Mail", id + ROWS + 1, &name))?;
        }
    }
    tx.commit()?;
    conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")?;
    assert_eq!(
        conn.query_row("SELECT count(*) FROM messages", [], |r| r
            .get::<_, usize>(0))?,
        ROWS
    );
    assert_eq!(
        conn.query_row("SELECT count(*) FROM vec_messages", [], |r| r
            .get::<_, usize>(0))?,
        ROWS
    );
    assert_eq!(
        conn.query_row("SELECT count(*) FROM message_folder_memberships", [], |r| r
            .get::<_, usize>(0))?,
        ROWS * 2
    );
    Ok(Fixture { directory, key })
}
