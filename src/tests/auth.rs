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

#[test]
fn verify_token_works() {
    use crate::auth::verify_token;
    use jsonwebtoken::{Algorithm, EncodingKey, Header, encode};
    // Par de claves P-256 solo para este test.
    const KEY: &str = "-----BEGIN PRIVATE KEY-----
MIGHAgEAMBMGByqGSM49AgEGCCqGSM49AwEHBG0wawIBAQQgC4J7MtH6foMLVEdN
CiR8Izk7CiGshbEPFJTCIGtgzZ6hRANCAASZfVFm5BLTHIJWEkBpwwgP2xSl9cQa
+4dtIQGH5JWPH0MnsdWfeBRBKvdXCNTWa6IZS5JCYz6gyll440niTHEw
-----END PRIVATE KEY-----";
    let jwks = serde_json::from_value(serde_json::json!({ "keys": [{
        "kty": "EC", "crv": "P-256", "kid": "k1", "alg": "ES256", "use": "sig",
        "x": "mX1RZuQS0xyCVhJAacMID9sUpfXEGvuHbSEBh-SVjx8", "y": "Qyex1Z94FEEq91cI1NZrohlLkkJjPqDKWXjjSeJMcTA"
    }]}))
    .unwrap();
    let iss = "https://p.supabase.co/auth/v1";
    let token = |kid: &str, claims: serde_json::Value| {
        let header = Header { kid: Some(kid.into()), ..Header::new(Algorithm::ES256) };
        encode(&header, &claims, &EncodingKey::from_ec_pem(KEY.as_bytes()).unwrap()).unwrap()
    };
    let claims = |aud: &str, exp: u64| serde_json::json!({ "sub": "00000000-0000-0000-0000-000000000001", "aud": aud, "iss": iss, "exp": exp });
    let future = 4_000_000_000;

    assert_eq!(verify_token(&token("k1", claims("authenticated", future)), &jwks, iss).unwrap().as_u128(), 1);
    assert!(verify_token(&token("k1", claims("authenticated", 1)), &jwks, iss).is_err(), "vencido");
    assert!(verify_token(&token("k1", claims("anon", future)), &jwks, iss).is_err(), "otra audiencia");
    assert!(verify_token(&token("k2", claims("authenticated", future)), &jwks, iss).is_err(), "kid desconocido");
    assert!(verify_token(&token("k1", claims("authenticated", future)), &jwks, "https://otro/auth/v1").is_err(), "otro emisor");
    // Mismo header y payload con la firma de otro token: debe rechazarse.
    let (a, b) = (token("k1", claims("authenticated", future)), token("k1", claims("authenticated", future + 1)));
    let forged = format!("{}.{}", &a[..a.rfind('.').unwrap()], &b[b.rfind('.').unwrap() + 1..]);
    assert!(verify_token(&forged, &jwks, iss).is_err(), "firma alterada");
}
