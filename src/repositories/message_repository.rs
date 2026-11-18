use sea_orm::{ConnectionTrait, DatabaseConnection, DbBackend, DbErr, QueryResult, Statement, prelude::Uuid};

use crate::domain::models::Message;
use crate::domain::repositories::{SQLiteRepository, message_repository::MessageRepository};

/// Usa la conexión SQLite de db::connect_sqlite, que crea la tabla.
pub struct SqliteMessageRepository(pub DatabaseConnection);

fn message_from_row(row: &QueryResult) -> Result<Message, DbErr> {
    Ok(Message { id: row.try_get("", "id")?, group_id: row.try_get("", "group_id")?, user_id: row.try_get("", "user_id")?, text: row.try_get("", "text")? })
}

impl SQLiteRepository<Message> for SqliteMessageRepository {
    type Pk = Uuid;

    // ON CONFLICT ... DO UPDATE conserva el rowid, que define el orden de get_messages.
    async fn save(&self, m: &Message) -> Result<(), DbErr> {
        let stmt = Statement::from_sql_and_values(
            DbBackend::Sqlite,
            "INSERT INTO messages (id, group_id, user_id, text) VALUES (?, ?, ?, ?) ON CONFLICT (id) DO UPDATE SET text = excluded.text",
            [m.id.into(), m.group_id.clone().into(), m.user_id.into(), m.text.clone().into()],
        );
        self.0.execute_raw(stmt).await.map(|_| ())
    }

    async fn get_by_pk(&self, id: &Uuid) -> Result<Option<Message>, DbErr> {
        let stmt = Statement::from_sql_and_values(DbBackend::Sqlite, "SELECT id, group_id, user_id, text FROM messages WHERE id = ?", [(*id).into()]);
        self.0.query_one_raw(stmt).await?.as_ref().map(message_from_row).transpose()
    }
}

impl MessageRepository for SqliteMessageRepository {
    // Un solo INSERT por lote: 4 parámetros por mensaje, lejos del límite de SQLite con lotes de 100.
    async fn save_all(&self, msgs: &[Message]) -> Result<(), DbErr> {
        if msgs.is_empty() {
            return Ok(());
        }
        let rows = vec!["(?, ?, ?, ?)"; msgs.len()].join(", ");
        let values = msgs.iter().flat_map(|m| [m.id.into(), m.group_id.into(), m.user_id.into(), m.text.clone().into()]);
        let stmt = Statement::from_sql_and_values(
            DbBackend::Sqlite,
            format!("INSERT INTO messages (id, group_id, user_id, text) VALUES {rows} ON CONFLICT (id) DO UPDATE SET text = excluded.text"),
            values,
        );
        self.0.execute_raw(stmt).await.map(|_| ())
    }

    async fn get_messages(&self, group_id: Uuid, limit: u64, offset: u64) -> Result<Vec<Message>, DbErr> {
        let stmt = Statement::from_sql_and_values(
            DbBackend::Sqlite,
            "SELECT id, group_id, user_id, text FROM messages WHERE group_id = ? ORDER BY rowid DESC LIMIT ? OFFSET ?",
            [group_id.into(), (limit as i64).into(), (offset as i64).into()],
        );
        self.0.query_all_raw(stmt).await?.iter().map(message_from_row).collect()
    }
}
