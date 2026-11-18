use crate::auth::parse_session;

#[test]
fn parse_session_works() {
    let v = serde_json::json!({
        "access_token": "a", "refresh_token": "r",
        "user": { "id": "00000000-0000-0000-0000-000000000001", "email": "a@b.c", "user_metadata": { "nickname": "ana", "name": "Ana" } }
    });
    let s = parse_session(&v).unwrap();
    assert_eq!((s.user_id.as_u128(), s.nickname.as_str(), s.refresh_token.as_str()), (1, "ana", "r"));

    let sin_nickname = serde_json::json!({ "refresh_token": "r", "user": { "id": "00000000-0000-0000-0000-000000000001", "email": "a@b.c" } });
    assert_eq!(parse_session(&sin_nickname).unwrap().nickname, "a@b.c", "sin nickname se usa el email");
    // Signup con confirmación de email: viene el usuario sin token.
    assert!(parse_session(&serde_json::json!({ "id": "00000000-0000-0000-0000-000000000001" })).is_err());
}
