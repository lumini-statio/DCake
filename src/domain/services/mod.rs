
pub mod group_service;
pub mod message_service;
pub mod user_service;
pub mod userxgroup_service;

use sea_orm::DbErr;

pub trait Service<T> {
    type Pk: ?Sized;
    async fn save(&self, item: &T) -> Result<(), DbErr>;
    async fn find_by_pk(&self, pk: &Self::Pk) -> Result<Option<T>, DbErr>;
}
