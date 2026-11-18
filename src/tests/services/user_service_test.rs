use std::cell::RefCell;

use sea_orm::{DbErr, prelude::Uuid};

use crate::domain::models::{Group, User};
use crate::domain::repositories::{PostgreSQLRepository, user_repository::UserRepository};
use crate::domain::services::Service;
use crate::services::user_service::UserServiceImpl;

/// Repo en memoria: guarda cada usuario que recibe para poder inspeccionarlo.
#[derive(Default)]
struct FakeRepo(RefCell<Vec<User>>);

impl PostgreSQLRepository<User> for FakeRepo {
    type Pk = Uuid;
    async fn save(&self, u: &User) -> Result<(), DbErr> {
        Ok(self.0.borrow_mut().push(u.clone()))
    }
    async fn get_by_pk(&self, _: &Uuid) -> Result<Option<User>, DbErr> {
        Ok(None)
    }
}

impl UserRepository for FakeRepo {
    async fn get_relations_by_user(&self, _: Uuid) -> Result<Vec<Group>, DbErr> {
        Ok(vec![])
    }
    async fn add_relation(&self, _: Uuid, _: Uuid) -> Result<(), DbErr> {
        Ok(())
    }
}

fn user() -> User {
    User {
        id: Uuid::from_u128(1),
        name: None,
        nickname: Some("ana".into()),
        description: None,
        email: None,
        is_admin: false,
        last_call: None,
        created_at: Default::default(),
    }
}

#[test]
fn save_guarda_el_usuario() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let svc = UserServiceImpl(FakeRepo::default());
    rt.block_on(svc.save(&user())).unwrap();
    assert_eq!(*svc.0.0.borrow(), [user()]);
}
