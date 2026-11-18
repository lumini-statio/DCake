use std::{env, time::Duration};

use sea_orm::{ConnectOptions, ConnectionTrait, Database, DatabaseConnection, DbErr};
use secrecy::{ExposeSecret, SecretString};

pub async fn connect() -> Result<DatabaseConnection, String> {
    dotenvy::dotenv().ok();

    let url = SecretString::from(
        env::var("DATABASE_URL").map_err(|e| format!("DATABASE_URL no definida: {e}"))?
        );
    let mut opt = ConnectOptions::new(url.expose_secret().to_owned());
    opt.connect_timeout(Duration::from_secs(5)).acquire_timeout(Duration::from_secs(5));
    Database::connect(opt).await.map_err(|e| e.to_string())
}

/// Abre (o crea) la base SQLite de mensajes, p. ej. "sqlite://messages.db?mode=rwc".
pub async fn connect_sqlite(url: &str) -> Result<DatabaseConnection, DbErr> {
    let db = Database::connect(url).await?;
    db.execute_unprepared(
        "CREATE TABLE IF NOT EXISTS messages (id BLOB PRIMARY KEY, group_id BLOB NOT NULL, user_id BLOB NOT NULL, text TEXT NOT NULL)",
    )
    .await?;
    Ok(db)
}
