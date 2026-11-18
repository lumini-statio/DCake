use sea_orm::{DbErr, prelude::Uuid};

use crate::domain::models::Group;

pub trait UserXGroupService {
    /// Grupos a los que pertenece el usuario.
    async fn get_relations_by_user(&self, user_id: Uuid) -> Result<Vec<Group>, DbErr>;

    /// Une al usuario al grupo; si ya pertenece no hace nada.
    async fn add_relation(&self, user_id: Uuid, group_id: Uuid) -> Result<(), DbErr>;
}
