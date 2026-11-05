//! Allowlisted, versioned logical snapshots. Never copy credentials or live work queues.
use std::io::{Read, Write};

use rusqlite::{params_from_iter, types::Value, Connection};
use serde::{Deserialize, Serialize};
use vault::backup::{BackupReader, BackupWriter};

use crate::{FtsTokenizerMode, SqliteMailStore, StorageError};

// INVARIANT: v11 adds only rebuildable indexes; the v1 backup remains schema-10 compatible.
const BACKUP_SCHEMA_VERSION: u32 = 10;

// INVARIANT: Order is dependency order. No identifier/SQL from the file is executed.
const TABLES: &[&str] = &[
    "accounts",
    "folders",
    "folder_sync_state",
    "messages",
    "attachments",
    "tags",
    "message_folder_memberships",
    "message_folder_overrides",
    "account_avatars",
    "message_categories",
    "category_corrections",
    "category_sender_overrides",
    "sender_review_preferences",
    "sender_review_choices",
    "sender_review_messages",
    "sender_review_frontiers",
    "due_date_dismissals",
    "alert_destinations",
    "alert_schedule_policies",
    "local_rules",
    "saved_views",
    "reminder_configs",
];
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    version: u32,
    schema: u32,
    tokenizer: FtsTokenizerMode,
}
#[derive(Serialize, Deserialize)]
enum Cell {
    Null,
    Integer(i64),
    Real(f64),
    Text(String),
    Blob(Vec<u8>),
}
impl Cell {
    fn value(self) -> Value {
        match self {
            Self::Null => Value::Null,
            Self::Integer(v) => Value::Integer(v),
            Self::Real(v) => Value::Real(v),
            Self::Text(v) => Value::Text(v),
            Self::Blob(v) => Value::Blob(v),
        }
    }
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Record {
    table: usize,
    cells: Option<Vec<Cell>>,
}
pub(crate) fn invalid() -> StorageError {
    StorageError::Query("invalid backup or portable definitions".into())
}
fn json<T: Serialize>(value: &T) -> Result<Vec<u8>, StorageError> {
    serde_json::to_vec(value).map_err(|_| invalid())
}
fn decode<T: serde::de::DeserializeOwned>(bytes: &[u8]) -> Result<T, StorageError> {
    serde_json::from_slice(bytes).map_err(|_| invalid())
}

impl SqliteMailStore {
    pub fn write_backup<W: Write>(&self, output: W, passphrase: &[u8]) -> Result<W, StorageError> {
        let mut writer = BackupWriter::new(output, passphrase)?;
        // INVARIANT: One read transaction pins every table to the same committed WAL
        // snapshot. A second connection's later commits cannot split this snapshot.
        self.transaction(|tx| {
            writer.record(&json(&Manifest {
                version: 1,
                schema: BACKUP_SCHEMA_VERSION,
                tokenizer: crate::settings::read_tokenizer(tx)?,
            })?)?;
            for (index, table) in TABLES.iter().enumerate() {
                write_table(tx, &mut writer, index, table)?;
                writer.record(&json(&Record {
                    table: index,
                    cells: None,
                })?)?;
            }
            Ok(())
        })?;
        Ok(writer.finish()?)
    }

    /// The caller must provide an unpublished, fresh encrypted database.
    pub fn read_backup<R: Read>(&self, input: R, passphrase: &[u8]) -> Result<(), StorageError> {
        let mut reader = BackupReader::new(input, passphrase)?;
        let manifest: Manifest = decode(&reader.record()?.ok_or_else(invalid)?)?;
        if manifest.version != 1 || manifest.schema != BACKUP_SCHEMA_VERSION {
            return Err(invalid());
        }
        self.transaction(|tx| {
            // Refuse even an empty-looking installation with configuration/history.
            let tables: Vec<String> = tx.prepare("SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%'")
                .map_err(|_| invalid())?.query_map([], |r| r.get(0)).map_err(|_| invalid())?
                .collect::<Result<_, _>>().map_err(|_| invalid())?;
            for table in tables {
                if matches!(table.as_str(), "schema_migrations" | "conversation_epoch" | "alert_schedule_cursor") || table.starts_with("messages_fts") || table.starts_with("vec_messages") { continue; }
                let count: i64 = tx.query_row(&format!("SELECT count(*) FROM \"{table}\""), [], |r| r.get(0)).map_err(|_| invalid())?;
                if count != 0 { return Err(invalid()); }
            }
            let mut table_index = 0;
            let mut count = 0u64;
            while let Some(bytes) = reader.record()? {
                let record: Record = decode(&bytes)?;
                if record.table != table_index || table_index >= TABLES.len() { return Err(invalid()); }
                if let Some(cells) = record.cells {
                    count += 1;
                    if count > 10_000_000 { return Err(invalid()); }
                    insert_row(tx, TABLES[table_index], cells)?;
                } else { table_index += 1; }
            }
            if table_index != TABLES.len() { return Err(invalid()); }
            sanitize(tx)?;
            if tx.prepare("PRAGMA foreign_key_check").map_err(|_| invalid())?.exists([]).map_err(|_| invalid())? { return Err(invalid()); }
            Ok(())
        })?;
        self.set_tokenizer(manifest.tokenizer)?;
        let conn = self.conn.lock().map_err(|_| invalid())?;
        let check: String = conn
            .query_row("PRAGMA integrity_check", [], |r| r.get(0))
            .map_err(|_| invalid())?;
        if check != "ok" {
            return Err(invalid());
        }
        Ok(())
    }
}
fn write_table<W: Write>(
    conn: &Connection,
    writer: &mut BackupWriter<W>,
    index: usize,
    table: &str,
) -> Result<(), StorageError> {
    let mut stmt = conn
        .prepare(&format!("SELECT * FROM {table}"))
        .map_err(|_| invalid())?;
    let columns = stmt.column_count();
    let mut rows = stmt.query([]).map_err(|_| invalid())?;
    while let Some(row) = rows.next().map_err(|_| invalid())? {
        let cells = (0..columns)
            .map(|i| {
                row.get::<_, Value>(i)
                    .map(|v| match v {
                        Value::Null => Cell::Null,
                        Value::Integer(v) => Cell::Integer(v),
                        Value::Real(v) => Cell::Real(v),
                        Value::Text(v) => Cell::Text(v),
                        Value::Blob(v) => Cell::Blob(v),
                    })
                    .map_err(|_| invalid())
            })
            .collect::<Result<Vec<_>, _>>()?;
        // Destination secret_key references are also excluded, even though opaque.
        let cells = safe_row(table, cells)?;
        writer.record(&json(&Record {
            table: index,
            cells: Some(cells),
        })?)?;
    }
    Ok(())
}
fn safe_row(table: &str, mut cells: Vec<Cell>) -> Result<Vec<Cell>, StorageError> {
    if table == "alert_destinations" {
        let Some(Cell::Text(definition)) = cells.get(2) else {
            return Err(invalid());
        };
        let mut destination: triage_ai::alerts::Destination = decode(definition.as_bytes())?;
        destination.enabled = false;
        destination.has_credentials = false;
        destination.public_origin = None;
        cells[2] = Cell::Text(serde_json::to_string(&destination).map_err(|_| invalid())?);
        cells[3] = Cell::Null;
        cells[4] = Cell::Integer(0);
    }
    if table == "alert_schedule_policies" {
        let Some(Cell::Text(definition)) = cells.get(2) else {
            return Err(invalid());
        };
        let mut policy: triage_ai::alerts::schedule::SchedulePolicy =
            decode(definition.as_bytes())?;
        if !policy.validate() {
            return Err(invalid());
        }
        policy.urgent_bypass = false;
        cells[2] = Cell::Text(serde_json::to_string(&policy).map_err(|_| invalid())?);
        cells[3] = Cell::Null;
        for cell in &mut cells[4..] {
            *cell = Cell::Integer(0);
        }
    }
    Ok(cells)
}
fn insert_row(conn: &Connection, table: &str, cells: Vec<Cell>) -> Result<(), StorageError> {
    let columns = conn
        .prepare(&format!("SELECT * FROM {table} LIMIT 0"))
        .map_err(|_| invalid())?
        .column_count();
    if cells.len() != columns {
        return Err(invalid());
    }
    let cells = safe_row(table, cells)?;
    let placeholders = vec!["?"; columns].join(",");
    conn.execute(
        &format!("INSERT INTO {table} VALUES ({placeholders})"),
        params_from_iter(cells.into_iter().map(Cell::value)),
    )
    .map_err(|_| invalid())?;
    Ok(())
}
fn sanitize(conn: &Connection) -> Result<(), StorageError> {
    // INVARIANT: Restored automation is inert. No job, lease, preview, authorization,
    // endpoint or consent is copied, and later startup cannot replay the source queues.
    for (table, kind) in [("local_rules", 0), ("reminder_configs", 1)] {
        let values: Vec<(String, String)> = conn
            .prepare(&format!("SELECT id,definition_json FROM {table}"))
            .map_err(|_| invalid())?
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
            .map_err(|_| invalid())?
            .collect::<Result<_, _>>()
            .map_err(|_| invalid())?;
        for (id, definition) in values {
            let definition = if kind == 0 {
                let mut value: triage_ai::rules::Rule = decode(definition.as_bytes())?;
                if !value.validate() {
                    return Err(invalid());
                }
                value.enabled = false;
                serde_json::to_string(&value)
            } else {
                let mut value: triage_ai::reminders::ScheduledReminderConfig =
                    decode(definition.as_bytes())?;
                if !value.validate() {
                    return Err(invalid());
                }
                value.enabled = false;
                serde_json::to_string(&value)
            }
            .map_err(|_| invalid())?;
            conn.execute(
                &format!("UPDATE {table} SET definition_json=?1 WHERE id=?2"),
                [&definition, &id],
            )
            .map_err(|_| invalid())?;
        }
    }
    let views: Vec<String> = conn
        .prepare("SELECT definition_json FROM saved_views")
        .map_err(|_| invalid())?
        .query_map([], |r| r.get(0))
        .map_err(|_| invalid())?
        .collect::<Result<_, _>>()
        .map_err(|_| invalid())?;
    for definition in views {
        let view: crate::saved_views::SavedView = decode(definition.as_bytes())?;
        if !view.validate() {
            return Err(invalid());
        }
    }
    conn.execute(
        "UPDATE sender_review_preferences SET enabled=0,hide_in_inbox=0,initializing=0",
        [],
    )
    .map_err(|_| invalid())?;
    Ok(())
}

#[cfg(test)]
mod tests;
