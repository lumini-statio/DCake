use sea_orm::{ConnectionTrait, DatabaseConnection, DbBackend, DbErr, Statement};
use sha2::{Digest, Sha256};

pub struct Profile {
    pub name: Option<String>,
    pub nickname: String,
    pub email: Option<String>,
}

pub fn password_hash(nickname: &str, password: &str) -> String {
    format!("{:x}", Sha256::digest(format!("{nickname}\0{password}")))
}

pub async fn register(db: &DatabaseConnection, p: &Profile, password: &str) -> Result<bool, DbErr> {
    let stmt = Statement::from_sql_and_values(
        DbBackend::Postgres,
        r#"INSERT INTO "users" (nickname, name, email, password) VALUES ($1, $2, $3, $4)
           ON CONFLICT (nickname) DO NOTHING RETURNING id"#,
        [p.nickname.clone().into(), p.name.clone().into(), p.email.clone().into(), password_hash(&p.nickname, password).into()],
    );
    Ok(db.query_one_raw(stmt).await?.is_some())
}

pub async fn login(db: &DatabaseConnection, nickname: &str, password: &str) -> Result<Option<Profile>, DbErr> {
    let stmt = Statement::from_sql_and_values(
        DbBackend::Postgres,
        r#"SELECT name, email FROM "users" WHERE nickname = $1 AND password = $2"#,
        [nickname.into(), password_hash(nickname, password).into()],
    );
    db.query_one_raw(stmt)
        .await?
        .map(|row| Ok(Profile { name: row.try_get("", "name")?, nickname: nickname.to_owned(), email: row.try_get("", "email")? }))
        .transpose()
}

#[test]
fn password_hash_works() {
    assert_eq!(password_hash("ana", "x"), password_hash("ana", "x"));
    assert_ne!(password_hash("ana", "x"), password_hash("ana", "y"));
    assert_ne!(password_hash("ana", "x"), password_hash("luis", "x"));
    assert_eq!(password_hash("ana", "x").len(), 64);
}
