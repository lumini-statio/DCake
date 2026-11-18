use sea_orm::prelude::Uuid;
use slint::SharedString;

use crate::join::validate_code;

#[test]
fn validate_code_works() {
    let id = "1675eb0a-6eaa-44e6-bc7a-9cb31647e000";
    let existing = [SharedString::from(""), SharedString::from(id)];
    let otro = "1675eb0a-6eaa-44e6-bc7a-9cb31647e001";
    assert_eq!(validate_code(&format!("  {otro} "), &existing), Ok(Uuid::parse_str(otro).unwrap()));
    assert!(validate_code("", &existing).is_err());
    assert!(validate_code("abc-1", &existing).is_err());
    assert!(validate_code(&id.to_uppercase(), &existing).is_err());
}
