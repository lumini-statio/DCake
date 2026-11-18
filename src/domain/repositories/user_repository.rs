use sea_orm::{DbErr, prelude::Uuid};

use super::PostgreSQLRepository;
use crate::domain::models::{Group, User};

pub trait UserRepository: PostgreSQLRepository<User, Pk = Uuid> {
    /// Grupos a los que pertenece el usuario.
    async fn get_relations_by_user(&self, user_id: Uuid) -> Result<Vec<Group>, DbErr>;

    /// Registra que el usuario pertenece al grupo; si ya pertenece no hace nada.
    async fn add_relation(&self, user_id: Uuid, group_id: Uuid) -> Result<(), DbErr>;
}
