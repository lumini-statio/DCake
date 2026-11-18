use sea_orm::{DbErr, prelude::Uuid};

use super::SQLiteRepository;
use crate::domain::models::Message;

pub trait MessageRepository: SQLiteRepository<Message, Pk = Uuid> {
    /// Mensajes del grupo, del más nuevo al más viejo.
    async fn get_messages(&self, group_id: Uuid, limit: u64, offset: u64) -> Result<Vec<Message>, DbErr>;
}
