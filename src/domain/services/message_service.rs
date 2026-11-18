use sea_orm::{DbErr, prelude::Uuid};

use super::Service;
use crate::domain::models::Message;

pub trait MessageService: Service<Message, Pk = Uuid> {
    /// Guarda (o actualiza) todos los mensajes en una sola sentencia.
    async fn save_all(&self, msgs: &[Message]) -> Result<(), DbErr>;
    /// Mensajes del grupo, del más nuevo al más viejo.
    async fn get_messages(&self, group_id: Uuid, limit: u64, offset: u64) -> Result<Vec<Message>, DbErr>;
}
