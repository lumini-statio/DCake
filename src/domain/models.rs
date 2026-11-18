use sea_orm::prelude::{DateTime, DateTimeWithTimeZone, Uuid};

/// Tabla "users".
#[derive(Debug, Clone, PartialEq)]
pub struct User {
    pub id: Uuid,
    pub name: Option<String>,
    pub nickname: Option<String>,
    pub description: Option<String>,
    pub email: Option<String>,
    pub is_admin: bool,
    pub last_call: Option<DateTime>,
    /// Lo pone la base (now()); save lo ignora.
    pub created_at: DateTimeWithTimeZone,
    /// Solo el hash, y solo de escritura: las lecturas lo dejan en None y save con None conserva el guardado.
    pub password_hash: Option<String>,
}

/// Tabla "groups".
#[derive(Debug, Clone, PartialEq)]
pub struct Group {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub last_call: Option<DateTime>,
    /// Lo pone la base (now()); save lo ignora.
    pub created_at: DateTimeWithTimeZone,
}

/// Tabla "messages" de SQLite.
#[derive(Debug, Clone, PartialEq)]
pub struct Message {
    pub id: Uuid,
    pub group_id: Uuid,
    pub user_id: Uuid,
    pub text: String,
}
