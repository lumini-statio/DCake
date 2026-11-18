use sea_orm::{DbErr, prelude::Uuid};

use crate::domain::models::Message;
use crate::domain::repositories::message_repository::MessageRepository;
use crate::domain::services::{Service, message_service::MessageService};

pub struct MessageServiceImpl<R>(pub R);

impl<R: MessageRepository> Service<Message> for MessageServiceImpl<R> {
    type Pk = Uuid;

    async fn save(&self, item: &Message) -> Result<(), DbErr> {
        self.0.save(item).await
    }

    async fn find_by_pk(&self, pk: &Uuid) -> Result<Option<Message>, DbErr> {
        self.0.get_by_pk(pk).await
    }
}

impl<R: MessageRepository> MessageService for MessageServiceImpl<R> {
    async fn save_all(&self, msgs: &[Message]) -> Result<(), DbErr> {
        self.0.save_all(msgs).await
    }

    async fn get_messages(&self, group_id: Uuid, limit: u64, offset: u64) -> Result<Vec<Message>, DbErr> {
        self.0.get_messages(group_id, limit, offset).await
    }
}
