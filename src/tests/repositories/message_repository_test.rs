use sea_orm::prelude::Uuid;

use crate::domain::models::Message;
use crate::domain::repositories::{SQLiteRepository, message_repository::MessageRepository};
use crate::repositories::message_repository::SqliteMessageRepository;

/// SQLite en memoria: no toca ninguna base real.
#[test]
fn guarda_busca_y_pagina_mensajes() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let repo = SqliteMessageRepository(rt.block_on(crate::db::connect_sqlite("sqlite::memory:")).unwrap());
    let user = Uuid::from_u128(7);
    let (g, otro) = (Uuid::from_u128(100), Uuid::from_u128(200));
    let m = |n: u128, g: Uuid, t: &str| Message { id: Uuid::from_u128(n), group_id: g, user_id: user, text: t.into() };
    for msg in [m(1, g, "hola"), m(2, g, "chau"), m(3, otro, "x"), m(1, g, "editado")] {
        rt.block_on(repo.save(&msg)).unwrap();
    }
    assert_eq!(rt.block_on(repo.get_by_pk(&Uuid::from_u128(1))).unwrap(), Some(m(1, g, "editado")));
    assert_eq!(rt.block_on(repo.get_by_pk(&Uuid::from_u128(9))).unwrap(), None);
    assert_eq!(rt.block_on(repo.get_messages(g, 1, 0)).unwrap(), vec![m(2, g, "chau")]);
    assert_eq!(rt.block_on(repo.get_messages(g, 10, 1)).unwrap(), vec![m(1, g, "editado")]);
}
