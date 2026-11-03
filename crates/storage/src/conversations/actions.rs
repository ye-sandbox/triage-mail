use super::*;
use crate::HybridQuery;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConversationAction {
    MarkRead,
    MarkUnread,
    Archive,
    Tag(String),
}
impl ConversationAction {
    fn validate(&self) -> Result<(), StorageError> {
        if let Self::Tag(tag) = self {
            if tag.trim().is_empty() || tag.len() > 128 || tag.chars().any(char::is_control) {
                return Err(stale());
            }
        }
        Ok(())
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationPreview {
    pub token: String,
    pub generation: String,
    pub action: ConversationAction,
    pub messages: Vec<MessageRowSummary>,
    pub outside_count: usize,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MutationStatus {
    pub message_id: String,
    pub pending: bool,
    pub failed: bool,
    pub attempts: usize,
}
impl SqliteMailStore {
    pub fn preview_conversation(
        &self,
        query: &HybridQuery,
        context: &ConversationQuery,
        id: &str,
        all: bool,
        action: ConversationAction,
    ) -> Result<ConversationPreview, StorageError> {
        action.validate()?;
        self.transaction(|tx| {
            let ids=super::search::matching_ids(tx,query,context,id,all)?;
            if ids.is_empty() || ids.len()>500 { return Err(stale()); }
            let matching=super::search::matching_ids(tx,query,context,id,false)?;
            let outside_count=ids.iter().filter(|id| !matching.contains(id)).count();
            let generation=epoch(tx)?;
            let token: String=tx.query_row("SELECT lower(hex(randomblob(16)))",[],|r|r.get(0)).map_err(error)?;
            tx.execute("DELETE FROM conversation_previews WHERE created_at<?1",[crate::now_epoch_ms()-300_000]).map_err(error)?;
            tx.execute("DELETE FROM conversation_previews WHERE token IN (SELECT token FROM conversation_previews ORDER BY created_at DESC,token LIMIT -1 OFFSET 15)",[]).map_err(error)?;
            tx.execute("INSERT INTO conversation_previews VALUES (?1,?2,?3,?4,?5)",params![token,generation,serde_json::to_string(&ids).map_err(error)?,serde_json::to_string(&action).map_err(error)?,crate::now_epoch_ms()]).map_err(error)?;
            Ok(ConversationPreview {token,generation,action,messages:crate::search::load_summaries(tx,&ids)?,outside_count})
        })
    }
    pub fn conversation_preview(
        &self,
        token: &str,
    ) -> Result<(String, Vec<String>, ConversationAction), StorageError> {
        if token.len() != 32 {
            return Err(stale());
        }
        self.transaction(|tx| {
            let (generation,ids,action):(String,String,String)=tx.query_row("SELECT generation,ids,action FROM conversation_previews WHERE token=?1 AND created_at>=?2",params![token,crate::now_epoch_ms()-300_000],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?))).optional().map_err(error)?.ok_or_else(stale)?;
            check_generation(tx,&generation)?;
            Ok((generation,serde_json::from_str(&ids).map_err(error)?,serde_json::from_str(&action).map_err(error)?))
        })
    }
    pub fn conversation_mutation_status(
        &self,
        ids: &[String],
    ) -> Result<Vec<MutationStatus>, StorageError> {
        if ids.len() > 500 {
            return Err(stale());
        }
        self.transaction(|tx| ids.iter().map(|id| {
            let (pending,failed,attempts)=tx.query_row("SELECT EXISTS(SELECT 1 FROM pending_mutations WHERE message_id=?1),COALESCE((SELECT failed FROM conversation_attempts WHERE message_id=?1),0),COALESCE((SELECT attempts FROM conversation_attempts WHERE message_id=?1),0)",[id],|r|Ok((r.get::<_,bool>(0)?,r.get::<_,bool>(1)?,r.get::<_,usize>(2)?))).map_err(error)?;
            Ok(MutationStatus {message_id:id.clone(),pending,failed:pending && failed,attempts})
        }).collect())
    }
    pub fn record_conversation_attempt(&self, id: &str, failed: bool) -> Result<(), StorageError> {
        self.transaction(|tx| {
            tx.execute("INSERT INTO conversation_attempts SELECT id,?2,1 FROM messages WHERE id=?1 ON CONFLICT(message_id) DO UPDATE SET failed=excluded.failed,attempts=attempts+1",params![id,failed]).map_err(error)?;
            Ok(())
        })
    }
}

/// INVARIANT: Even headless callers cannot substitute IDs/actions into a native preview.
pub(crate) fn consume_preview(
    tx: &Connection,
    token: &str,
    generation: &str,
    changes: &[(String, crate::desktop::LocalMutation<'_>, Option<String>)],
) -> Result<(), StorageError> {
    use crate::desktop::LocalMutation;
    let row:Option<(String,String)>=tx.query_row("SELECT ids,action FROM conversation_previews WHERE token=?1 AND generation=?2 AND created_at>=?3",params![token,generation,crate::now_epoch_ms()-300_000],|r|Ok((r.get(0)?,r.get(1)?))).optional().map_err(error)?;
    let (ids, action) = row.ok_or_else(stale)?;
    let ids: Vec<String> = serde_json::from_str(&ids).map_err(error)?;
    let action: ConversationAction = serde_json::from_str(&action).map_err(error)?;
    let same_action = |local: &LocalMutation<'_>| match (&action, local) {
        (ConversationAction::MarkRead, LocalMutation::Read(true))
        | (ConversationAction::MarkUnread, LocalMutation::Read(false))
        | (ConversationAction::Archive, LocalMutation::Folder("Archive")) => true,
        (ConversationAction::Tag(expected), LocalMutation::Tag(actual)) => expected == actual,
        _ => false,
    };
    if ids.len() != changes.len()
        || ids
            .iter()
            .zip(changes)
            .any(|(id, (actual, local, _))| id != actual || !same_action(local))
    {
        return Err(stale());
    }
    tx.execute("DELETE FROM conversation_previews WHERE token=?1", [token])
        .map_err(error)?;
    Ok(())
}
