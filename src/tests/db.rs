use sea_orm::ConnectionTrait;

use crate::db::connect;

// Prueba real contra la base del .env: cargo test -- --ignored
#[test]
#[ignore]
fn connect_works() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let db = rt.block_on(connect()).unwrap();
    rt.block_on(db.execute_unprepared("SELECT 1")).unwrap();
}
