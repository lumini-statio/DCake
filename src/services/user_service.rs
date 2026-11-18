use sea_orm::{DbErr, prelude::Uuid};

use crate::auth::password_hash;
use crate::domain::models::User;
use crate::domain::repositories::user_repository::UserRepository;
use crate::domain::services::{Service, user_service::UserService};

pub struct UserServiceImpl<R>(pub R);

impl<R: UserRepository> Service<User> for UserServiceImpl<R> {
    type Pk = Uuid;

    /// Nunca escribe la contraseña: solo save_with_password calcula y guarda el hash.
    async fn save(&self, item: &User) -> Result<(), DbErr> {
        self.0.save(&User { password_hash: None, ..item.clone() }).await
    }

    async fn find_by_pk(&self, pk: &Uuid) -> Result<Option<User>, DbErr> {
        self.0.get_by_pk(pk).await
    }
}

impl<R: UserRepository> UserService for UserServiceImpl<R> {
    async fn save_with_password(&self, user: &User, password: &str) -> Result<(), DbErr> {
        // Sin nickname no hay con qué iniciar sesión: la contraseña quedaría inservible.
        if user.nickname.as_deref().is_none_or(str::is_empty) {
            return Err(DbErr::Custom("el usuario necesita nickname para guardar la contraseña".into()));
        }
        if password.is_empty() {
            return Err(DbErr::Custom("la contraseña está vacía".into()));
        }
        self.0.save(&User { password_hash: Some(password_hash(password)), ..user.clone() }).await
    }
}
