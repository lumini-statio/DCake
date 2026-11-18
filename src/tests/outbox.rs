use std::sync::Arc;

use sea_orm::{Database, prelude::Uuid};

use crate::domain::models::Message;
use crate::domain::repositories::message_repository::MessageRepository;
use crate::outbox::{Pending, flush};
use crate::repositories::message_repository::SqliteMessageRepository;
use crate::services::message_service::MessageServiceImpl;

fn pending(n: u128, g: Uuid) -> Pending {
    let msgs = (0..n).map(|i| Message { id: Uuid::from_u128(i + 1), group_id: g, user_id: Uuid::from_u128(7), text: i.to_string() });
    Pending::new(msgs.collect::<Vec<_>>().into())
}

/// SQLite en memoria: no toca ninguna base real.
#[test]
fn flush_guarda_todo_en_lotes_y_vacia_pendientes() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let db = rt.block_on(crate::db::connect_sqlite("sqlite::memory:")).unwrap();
    let g = Uuid::from_u128(100);
    let p = pending(250, g);
    flush(rt.handle().clone(), Arc::new(MessageServiceImpl(SqliteMessageRepository(db.clone()))), p.clone()).join().unwrap();
    assert!(p.lock().unwrap().is_empty());
    let saved = rt.block_on(SqliteMessageRepository(db).get_messages(g, 300, 0)).unwrap();
    assert_eq!(saved.len(), 250);
    assert_eq!(saved[0].text, "249"); // conserva el orden de llegada
}

#[test]
fn flush_con_error_deja_los_pendientes() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    // Sin connect_sqlite no existe la tabla: el guardado falla.
    let db = rt.block_on(Database::connect("sqlite::memory:")).unwrap();
    let p = pending(5, Uuid::from_u128(100));
    flush(rt.handle().clone(), Arc::new(MessageServiceImpl(SqliteMessageRepository(db))), p.clone()).join().unwrap();
    assert_eq!(p.lock().unwrap().len(), 5);
}
