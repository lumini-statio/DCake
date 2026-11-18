use sea_orm::{ConnectionTrait, DatabaseConnection, DbBackend, DbErr, QueryResult, Statement, prelude::Uuid};

use super::group_repository::{GROUP_COLUMNS, group_from_row};
use crate::domain::models::{Group, User};
use crate::domain::repositories::{PostgreSQLRepository, user_repository::UserRepository};

pub struct PgUserRepository(pub DatabaseConnection);

/// Columnas de "users" con alias `u`, en el orden que lee user_from_row.
pub(super) const USER_COLUMNS: &str = "u.id, u.name, u.nickname, u.description, u.email, u.is_admin, u.last_call, u.created_at";

pub(super) fn user_from_row(row: &QueryResult) -> Result<User, DbErr> {
    Ok(User {
        id: row.try_get("", "id")?,
        name: row.try_get("", "name")?,
        nickname: row.try_get("", "nickname")?,
        description: row.try_get("", "description")?,
        email: row.try_get("", "email")?,
        is_admin: row.try_get("", "is_admin")?,
        last_call: row.try_get("", "last_call")?,
        created_at: row.try_get("", "created_at")?,
    })
}

impl PostgreSQLRepository<User> for PgUserRepository {
    type Pk = Uuid;

    /// Una sola sentencia; si el nickname (UNIQUE) ya existe, actualiza esa fila y conserva su id.
    /// No toca created_at.
    async fn save(&self, u: &User) -> Result<(), DbErr> {
        let stmt = Statement::from_sql_and_values(
            DbBackend::Postgres,
            r#"INSERT INTO users (id, name, nickname, description, email, is_admin, last_call) VALUES ($1, $2, $3, $4, $5, $6, $7)
               ON CONFLICT (nickname) DO UPDATE SET name = $2, description = $4, email = $5, is_admin = $6, last_call = $7"#,
            [
                u.id.into(),
                u.name.clone().into(),
                u.nickname.clone().into(),
                u.description.clone().into(),
                u.email.clone().into(),
                u.is_admin.into(),
                u.last_call.into(),
            ],
        );
        self.0.execute_raw(stmt).await.map(|_| ())
    }

    async fn get_by_pk(&self, id: &Uuid) -> Result<Option<User>, DbErr> {
        let stmt = Statement::from_sql_and_values(DbBackend::Postgres, format!("SELECT {USER_COLUMNS} FROM users u WHERE u.id = $1"), [(*id).into()]);
        self.0.query_one_raw(stmt).await?.as_ref().map(user_from_row).transpose()
    }
}

impl UserRepository for PgUserRepository {
    async fn get_relations_by_user(&self, user_id: Uuid) -> Result<Vec<Group>, DbErr> {
        // "user" y "group" son palabras reservadas: van entre comillas.
        let stmt = Statement::from_sql_and_values(
            DbBackend::Postgres,
            format!(r#"SELECT {GROUP_COLUMNS} FROM groups g JOIN users_x_groups ug ON ug."group" = g.id WHERE ug."user" = $1 ORDER BY g.name"#),
            [user_id.into()],
        );
        self.0.query_all_raw(stmt).await?.iter().map(group_from_row).collect()
    }

    async fn add_relation(&self, user_id: Uuid, group_id: Uuid) -> Result<(), DbErr> {
        // NOT EXISTS evita duplicados sin depender de una restricción UNIQUE en la tabla.
        let stmt = Statement::from_sql_and_values(
            DbBackend::Postgres,
            r#"INSERT INTO users_x_groups ("user", "group") SELECT $1, $2
               WHERE NOT EXISTS (SELECT 1 FROM users_x_groups WHERE "user" = $1 AND "group" = $2)"#,
            [user_id.into(), group_id.into()],
        );
        self.0.execute_raw(stmt).await.map(|_| ())
    }
}
