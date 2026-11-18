use sea_orm::{DbErr, prelude::Uuid};

use crate::domain::models::User;
use crate::domain::repositories::user_repository::UserRepository;
use crate::domain::services::Service;

pub struct UserServiceImpl<R>(pub R);

impl<R: UserRepository> Service<User> for UserServiceImpl<R> {
    type Pk = Uuid;

    async fn save(&self, item: &User) -> Result<(), DbErr> {
        self.0.save(item).await
    }

    async fn find_by_pk(&self, pk: &Uuid) -> Result<Option<User>, DbErr> {
        self.0.get_by_pk(pk).await
    }
}
