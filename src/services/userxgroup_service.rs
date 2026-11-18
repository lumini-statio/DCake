use sea_orm::{DbErr, prelude::Uuid};

use crate::domain::models::Group;
use crate::domain::repositories::user_repository::UserRepository;
use crate::domain::services::userxgroup_service::UserXGroupService;

pub struct UserXGroupServiceImpl<R>(pub R);

impl<R: UserRepository> UserXGroupService for UserXGroupServiceImpl<R> {
    async fn get_relations_by_user(&self, user_id: Uuid) -> Result<Vec<Group>, DbErr> {
        self.0.get_relations_by_user(user_id).await
    }

    async fn add_relation(&self, user_id: Uuid, group_id: Uuid) -> Result<(), DbErr> {
        self.0.add_relation(user_id, group_id).await
    }
}
