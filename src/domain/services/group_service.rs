use sea_orm::{DbErr, prelude::Uuid};

use super::Service;
use crate::domain::models::{Group, User};

pub trait GroupService: Service<Group, Pk = Uuid> {
    async fn get_participants(&self, group_id: Uuid, limit: u64, offset: u64) -> Result<Vec<User>, DbErr>;
}
