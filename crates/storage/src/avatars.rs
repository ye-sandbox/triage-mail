//! Account photos live only in the encrypted database, separately from account metadata.
use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};

use crate::{SqliteMailStore, StorageError};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountAvatar {
    pub bytes: Vec<u8>,
    pub mime_type: String,
    pub width: u32,
    pub height: u32,
}

impl SqliteMailStore {
    pub fn account_avatar(&self, account_id: &str) -> Result<Option<AccountAvatar>, StorageError> {
        let conn = self.conn.lock().map_err(|_| error())?;
        conn.query_row(
            "SELECT avatar_bytes,mime_type,width,height FROM account_avatars WHERE account_id=?1",
            [account_id],
            |row| {
                Ok(AccountAvatar {
                    bytes: row.get(0)?,
                    mime_type: row.get(1)?,
                    width: row.get(2)?,
                    height: row.get(3)?,
                })
            },
        )
        .optional()
        .map_err(|_| error())
    }

    pub fn save_account_avatar(
        &self,
        account_id: &str,
        avatar: &AccountAvatar,
    ) -> Result<(), StorageError> {
        // INVARIANT: Only normalized thumbnails reach encrypted persistence; native code decodes originals.
        if avatar.mime_type != "image/png"
            || avatar.width != 128
            || avatar.height != 128
            || avatar.bytes.is_empty()
            || avatar.bytes.len() > 128 * 1024
        {
            return Err(error());
        }
        self.write_transaction(
            |tx| {
                tx.execute(
                    "INSERT INTO account_avatars(account_id,avatar_bytes,mime_type,width,height,updated_at_epoch_ms)
                     VALUES (?1,?2,?3,?4,?5,?6) ON CONFLICT(account_id) DO UPDATE SET
                     avatar_bytes=excluded.avatar_bytes,mime_type=excluded.mime_type,width=excluded.width,
                     height=excluded.height,updated_at_epoch_ms=excluded.updated_at_epoch_ms",
                    params![account_id, avatar.bytes, avatar.mime_type, avatar.width, avatar.height, crate::now_epoch_ms()],
                ).map_err(|_| error())?;
                Ok(())
            },
            |_, _, _| Ok(()),
        )
    }

    pub fn remove_account_avatar(&self, account_id: &str) -> Result<(), StorageError> {
        self.write_transaction(
            |tx| {
                tx.execute(
                    "DELETE FROM account_avatars WHERE account_id=?1",
                    [account_id],
                )
                .map_err(|_| error())?;
                Ok(())
            },
            |_, _, _| Ok(()),
        )
    }
}

fn error() -> StorageError {
    StorageError::Query("account photo unavailable".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AccountRecord, DatabaseKey};

    #[test]
    fn photos_are_encrypted_account_bound_and_survive_reopen() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("mail.db");
        let key = DatabaseKey::from_bytes([31; 32]);
        let store = SqliteMailStore::open(&path, &key, Default::default()).unwrap();
        for id in ["a", "b"] {
            store
                .insert_account(&AccountRecord {
                    id: id.into(),
                    email: format!("{id}@example.com"),
                    auth_type: "synthetic".into(),
                    created_at_epoch_ms: 0,
                    updated_at_epoch_ms: 0,
                })
                .unwrap();
        }
        let mut avatar = AccountAvatar {
            bytes: vec![42; 64],
            mime_type: "image/png".into(),
            width: 128,
            height: 128,
        };
        store.save_account_avatar("a", &avatar).unwrap();
        assert_eq!(store.account_avatar("b").unwrap(), None);
        assert!(store.save_account_avatar("unknown", &avatar).is_err());
        avatar.bytes = vec![43; 32];
        store.save_account_avatar("a", &avatar).unwrap();
        drop(store);
        assert!(!std::fs::read(&path)
            .unwrap()
            .starts_with(b"SQLite format 3"));
        assert!(SqliteMailStore::open(
            &path,
            &DatabaseKey::from_bytes([32; 32]),
            Default::default()
        )
        .is_err());
        let store = SqliteMailStore::open(&path, &key, Default::default()).unwrap();
        assert_eq!(store.account_avatar("a").unwrap(), Some(avatar));
        store.remove_account_avatar("a").unwrap();
        store.remove_account_avatar("a").unwrap();
        drop(store);
        let store = SqliteMailStore::open(&path, &key, Default::default()).unwrap();
        assert_eq!(store.account_avatar("a").unwrap(), None);
    }
}
