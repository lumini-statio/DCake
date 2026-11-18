use sea_orm::{ConnectionTrait, DatabaseConnection, DbBackend, DbErr, QueryResult, Statement, prelude::Uuid};

use super::user_repository::{USER_COLUMNS, user_from_row};
use crate::domain::models::{Group, User};
use crate::domain::repositories::{PostgreSQLRepository, group_repository::GroupRepository};

pub struct PgGroupRepository(pub DatabaseConnection);

/// Columnas de "groups" con alias `g`, en el orden que lee group_from_row.
pub(super) const GROUP_COLUMNS: &str = "g.id, g.name, g.description, g.last_call, g.created_at";

pub(super) fn group_from_row(row: &QueryResult) -> Result<Group, DbErr> {
    Ok(Group {
        id: row.try_get("", "id")?,
        name: row.try_get("", "name")?,
        description: row.try_get("", "description")?,
        last_call: row.try_get("", "last_call")?,
        created_at: row.try_get("", "created_at")?,
    })
}

impl PostgreSQLRepository<Group> for PgGroupRepository {
    type Pk = Uuid;

    /// No toca created_at: lo pone la base.
    async fn save(&self, g: &Group) -> Result<(), DbErr> {
        let stmt = Statement::from_sql_and_values(
            DbBackend::Postgres,
            "INSERT INTO groups (id, name, description, last_call) VALUES ($1, $2, $3, $4)
             ON CONFLICT (id) DO UPDATE SET name = $2, description = $3, last_call = $4",
            [g.id.into(), g.name.clone().into(), g.description.clone().into(), g.last_call.into()],
        );
        self.0.execute_raw(stmt).await.map(|_| ())
    }

    async fn get_by_pk(&self, id: &Uuid) -> Result<Option<Group>, DbErr> {
        let stmt = Statement::from_sql_and_values(DbBackend::Postgres, format!("SELECT {GROUP_COLUMNS} FROM groups g WHERE g.id = $1"), [(*id).into()]);
        self.0.query_one_raw(stmt).await?.as_ref().map(group_from_row).transpose()
    }
}

impl GroupRepository for PgGroupRepository {
    async fn get_participants(&self, group_id: Uuid, limit: u64, offset: u64) -> Result<Vec<User>, DbErr> {
        // "user" y "group" son palabras reservadas: van entre comillas.
        let stmt = Statement::from_sql_and_values(
            DbBackend::Postgres,
            format!(
                r#"SELECT {USER_COLUMNS} FROM users u JOIN users_x_groups ug ON ug."user" = u.id
                   WHERE ug."group" = $1 ORDER BY u.nickname LIMIT $2 OFFSET $3"#
            ),
            [group_id.into(), (limit as i64).into(), (offset as i64).into()],
        );
        self.0.query_all_raw(stmt).await?.iter().map(user_from_row).collect()
    }
}
