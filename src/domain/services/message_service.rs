use sea_orm::{DbErr, prelude::Uuid};

use super::Service;
use crate::domain::models::Message;

pub trait MessageService: Service<Message, Pk = Uuid> {
    /// Mensajes del grupo, del más nuevo al más viejo.
    async fn get_messages(&self, group_id: Uuid, limit: u64, offset: u64) -> Result<Vec<Message>, DbErr>;
}
