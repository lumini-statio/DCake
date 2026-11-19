use jsonwebtoken::{Algorithm, DecodingKey, Validation, jwk::JwkSet};
use sea_orm::prelude::Uuid;
use serde_json::{Value, json};

/// Sesión de Supabase Auth. El refresh token es lo que se guarda en el keyring.
pub struct Session {
    pub user_id: Uuid,
    pub nickname: String,
    pub refresh_token: String,
}

fn env(name: &str) -> Result<String, String> {
    dotenvy::dotenv().ok();
    std::env::var(name).map_err(|_| format!("{name} no definida."))
}

async fn send(path: &str, body: Value) -> Result<(reqwest::StatusCode, Value), String> {
    let url = env("SUPABASE_URL")?;
    let key = env("SUPABASE_PUBLISHABLE_KEY")?;
    let res = reqwest::Client::new()
        .post(format!("{}/auth/v1/{path}", url.trim_end_matches('/')))
        .header("apikey", key)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Sin conexión con Supabase: {e}"))?;
    let status = res.status();
    Ok((status, res.json().await.unwrap_or_default()))
}

async fn post(path: &str, body: Value) -> Result<Value, String> {
    let (status, v) = send(path, body).await?;
    if status.is_success() {
        return Ok(v);
    }
    let msg = ["msg", "error_description", "message"].iter().find_map(|k| v[*k].as_str());
    Err(msg.map_or_else(|| format!("Error de Supabase ({status})."), str::to_owned))
}

pub fn parse_session(v: &Value) -> Result<Session, String> {
    let user = &v["user"];
    let user_id = user["id"].as_str().and_then(|s| Uuid::parse_str(s).ok()).ok_or("Respuesta de Supabase sin usuario.")?;
    let refresh_token = v["refresh_token"].as_str().ok_or("Respuesta de Supabase sin token.")?.to_owned();
    // Usuarios creados fuera de la app pueden no tener nickname: se usa el email, que también es único.
    let nickname = user["user_metadata"]["nickname"].as_str().or(user["email"].as_str()).ok_or("Respuesta de Supabase sin nickname ni email.")?.to_owned();
    Ok(Session { user_id, nickname, refresh_token })
}

pub fn verify_token(token: &str, jwks: &JwkSet, issuer: &str) -> Result<Uuid, String> {
    let invalid = |e: jsonwebtoken::errors::Error| format!("Access token inválido: {e}");
    let header = jsonwebtoken::decode_header(token).map_err(invalid)?;
    // Solo algoritmos asimétricos: un HS256 no se puede verificar con el JWKS.
    if !matches!(header.alg, Algorithm::ES256 | Algorithm::RS256) {
        return Err(format!("Algoritmo de token no soportado: {:?}.", header.alg));
    }
    let jwk = header.kid.as_deref().and_then(|k| jwks.find(k)).ok_or("Token firmado con una clave desconocida.")?;
    let key = DecodingKey::from_jwk(jwk).map_err(invalid)?;
    let mut validation = Validation::new(header.alg);
    validation.set_audience(&["authenticated"]);
    validation.set_issuer(&[issuer]);
    let claims = jsonwebtoken::decode::<Value>(token, &key, &validation).map_err(invalid)?.claims;
    claims["sub"].as_str().and_then(|s| Uuid::parse_str(s).ok()).ok_or_else(|| "Access token sin usuario.".into())
}

async fn verified(v: Value) -> Result<Session, String> {
    let s = parse_session(&v)?;
    let token = v["access_token"].as_str().ok_or("Respuesta de Supabase sin access token.")?;
    let url = env("SUPABASE_JWKS_URL")?;
    let jwks: JwkSet = async { reqwest::get(url).await?.error_for_status()?.json().await }
        .await
        .map_err(|e: reqwest::Error| format!("No se pudo obtener el JWKS de Supabase: {e}"))?;
    let issuer = format!("{}/auth/v1", env("SUPABASE_URL")?.trim_end_matches('/'));
    if verify_token(token, &jwks, &issuer)? != s.user_id {
        return Err("El access token no corresponde al usuario.".into());
    }
    Ok(s)
}

pub async fn login(email: &str, password: &str) -> Result<Session, String> {
    verified(post("token?grant_type=password", json!({ "email": email, "password": password })).await?).await
}

pub async fn register(nickname: &str, name: Option<&str>, email: &str, password: &str) -> Result<Option<Session>, String> {
    let v = post("signup", json!({ "email": email, "password": password, "data": { "nickname": nickname, "name": name } })).await?;
    if v["refresh_token"].is_null() { Ok(None) } else { verified(v).await.map(Some) }
}

pub async fn refresh(refresh_token: &str) -> Result<Option<Session>, String> {
    let (status, v) = send("token?grant_type=refresh_token", json!({ "refresh_token": refresh_token })).await?;
    // 400/403 es token inválido; otros errores (401 apikey mala, 429, 5xx) no significan que la sesión venció.
    if matches!(status.as_u16(), 400 | 403) {
        return Ok(None);
    }
    if !status.is_success() {
        return Err(format!("Error de Supabase ({status})."));
    }
    verified(v).await.map(Some)
}

// Una entrada por cuenta, indexada por nickname (único).
fn entry(nickname: &str) -> Result<keyring::Entry, String> {
    keyring::Entry::new("jaw-jo", nickname).map_err(|e| format!("Keyring: {e}"))
}

pub fn save_token(nickname: &str, token: &str) -> Result<(), String> {
    entry(nickname)?.set_password(token).map_err(|e| format!("Keyring: {e}"))
}

pub fn load_token(nickname: &str) -> Option<String> {
    entry(nickname).ok()?.get_password().ok()
}

pub fn delete_token(nickname: &str) {
    let _ = entry(nickname).map(|e| e.delete_credential());
}
