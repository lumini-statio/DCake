use sea_orm::{DbErr, prelude::Uuid};

use crate::domain::models::{Group, User};
use crate::domain::repositories::group_repository::GroupRepository;
use crate::domain::services::{Service, group_service::GroupService};

pub struct GroupServiceImpl<R>(pub R);

impl<R: GroupRepository> Service<Group> for GroupServiceImpl<R> {
    type Pk = Uuid;

    async fn save(&self, item: &Group) -> Result<(), DbErr> {
        self.0.save(item).await
    }

    async fn find_by_pk(&self, pk: &Uuid) -> Result<Option<Group>, DbErr> {
        self.0.get_by_pk(pk).await
    }
}

impl<R: GroupRepository> GroupService for GroupServiceImpl<R> {
    async fn get_participants(&self, group_id: Uuid, limit: u64, offset: u64) -> Result<Vec<User>, DbErr> {
        self.0.get_participants(group_id, limit, offset).await
    }
}
