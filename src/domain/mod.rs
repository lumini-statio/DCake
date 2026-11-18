// Las interfaces solo se usan dentro del crate: no hace falta exigir futures Send.
#![allow(async_fn_in_trait)]

pub mod models;
pub mod repositories;
pub mod services;
