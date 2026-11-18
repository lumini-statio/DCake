use sea_orm::{DbErr, prelude::Uuid};

use super::Service;
use crate::domain::models::User;

pub trait UserService: Service<User, Pk = Uuid> {
    /// Guarda el usuario y el hash argon2 de `password` en una sola escritura. Requiere nickname para poder iniciar sesión.
    async fn save_with_password(&self, user: &User, password: &str) -> Result<(), DbErr>;
}
