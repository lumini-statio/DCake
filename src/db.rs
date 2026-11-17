use std::{env, time::Duration};

use sea_orm::{ConnectOptions, ConnectionTrait, Database, DatabaseConnection, DbBackend, DbErr, Statement};
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

/// Busca en la tabla "groups" el grupo cuyo id es `code`; devuelve su nombre.
pub async fn find_group_name(db: &DatabaseConnection, code: &str) -> Result<Option<String>, DbErr> {
    // id::text compara sin depender del tipo de la columna (int, uuid o texto).
    let statement = Statement::from_sql_and_values(DbBackend::Postgres, "SELECT name FROM groups WHERE id::text = $1", [code.into()]);
    db.query_one_raw(statement).await?.map(|row| row.try_get("", "name")).transpose()
}

// Prueba real contra la base del .env: cargo test -- --ignored
#[test]
#[ignore]
fn connect_works() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let db = rt.block_on(connect()).unwrap();
    rt.block_on(db.execute_unprepared("SELECT 1")).unwrap();
}
