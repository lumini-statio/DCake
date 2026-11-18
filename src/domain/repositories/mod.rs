pub mod group_repository;
pub mod message_repository;
pub mod user_repository;

use sea_orm::DbErr;

pub trait PostgreSQLRepository<T> {
    type Pk: ?Sized;
    async fn save(&self, item: &T) -> Result<(), DbErr>;
    async fn get_by_pk(&self, pk: &Self::Pk) -> Result<Option<T>, DbErr>;
}

pub trait SQLiteRepository<T> {
    type Pk: ?Sized;
    async fn save(&self, item: &T) -> Result<(), DbErr>;
    async fn get_by_pk(&self, pk: &Self::Pk) -> Result<Option<T>, DbErr>;
}
