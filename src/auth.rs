use sea_orm::prelude::Uuid;
use serde_json::{Value, json};

/// Sesión de Supabase Auth. El refresh token es lo que se guarda en el keyring.
pub struct Session {
    pub user_id: Uuid,
    pub nickname: String,
    pub refresh_token: String,
}

/// POST a `{SUPABASE_URL}/auth/v1/{path}`; Err solo si no se pudo hablar con Supabase.
async fn send(path: &str, body: Value) -> Result<(reqwest::StatusCode, Value), String> {
    dotenvy::dotenv().ok();
    let url = std::env::var("SUPABASE_URL").map_err(|_| "SUPABASE_URL no definida.")?;
    let key = std::env::var("SUPABASE_PUBLISHABLE_KEY").map_err(|_| "SUPABASE_PUBLISHABLE_KEY no definida.")?;
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

/// Como `send`, pero una respuesta de error de Supabase también es Err, con su mensaje.
async fn post(path: &str, body: Value) -> Result<Value, String> {
    let (status, v) = send(path, body).await?;
    if status.is_success() {
        return Ok(v);
    }
    // Según la versión de GoTrue el mensaje viene en "msg", "error_description" o "message".
    let msg = ["msg", "error_description", "message"].iter().find_map(|k| v[*k].as_str());
    Err(msg.map_or_else(|| format!("Error de Supabase ({status})."), str::to_owned))
}

/// Arma la sesión desde la respuesta de /token o /signup.
pub fn parse_session(v: &Value) -> Result<Session, String> {
    let user = &v["user"];
    let user_id = user["id"].as_str().and_then(|s| Uuid::parse_str(s).ok()).ok_or("Respuesta de Supabase sin usuario.")?;
    let refresh_token = v["refresh_token"].as_str().ok_or("Respuesta de Supabase sin token.")?.to_owned();
    // Usuarios creados fuera de la app pueden no tener nickname: se usa el email, que también es único.
    let nickname = user["user_metadata"]["nickname"].as_str().or(user["email"].as_str()).ok_or("Respuesta de Supabase sin nickname ni email.")?.to_owned();
    Ok(Session { user_id, nickname, refresh_token })
}

pub async fn login(email: &str, password: &str) -> Result<Session, String> {
    parse_session(&post("token?grant_type=password", json!({ "email": email, "password": password })).await?)
}

/// None si Supabase pide confirmar el email antes de dar sesión.
pub async fn register(nickname: &str, name: Option<&str>, email: &str, password: &str) -> Result<Option<Session>, String> {
    let v = post("signup", json!({ "email": email, "password": password, "data": { "nickname": nickname, "name": name } })).await?;
    if v["refresh_token"].is_null() { Ok(None) } else { parse_session(&v).map(Some) }
}

/// Ok(None) si la sesión venció: Supabase rechaza el token (vencido, revocado o ya usado).
/// Supabase rota el refresh token: hay que guardar el de la sesión devuelta.
pub async fn refresh(refresh_token: &str) -> Result<Option<Session>, String> {
    let (status, v) = send("token?grant_type=refresh_token", json!({ "refresh_token": refresh_token })).await?;
    // 400/403 es token inválido; otros errores (401 apikey mala, 429, 5xx) no significan que la sesión venció.
    if matches!(status.as_u16(), 400 | 403) {
        return Ok(None);
    }
    if !status.is_success() {
        return Err(format!("Error de Supabase ({status})."));
    }
    parse_session(&v).map(Some)
}

/// Una entrada por cuenta, indexada por nickname (único).
fn entry(nickname: &str) -> Result<keyring::Entry, String> {
    keyring::Entry::new("jaw-yo", nickname).map_err(|e| format!("Keyring: {e}"))
}

pub fn save_token(nickname: &str, token: &str) -> Result<(), String> {
    entry(nickname)?.set_password(token).map_err(|e| format!("Keyring: {e}"))
}

/// None si no hay token guardado o el keyring no está disponible.
pub fn load_token(nickname: &str) -> Option<String> {
    entry(nickname).ok()?.get_password().ok()
}

pub fn delete_token(nickname: &str) {
    // Si no había token no hay nada que borrar: el error se ignora.
    let _ = entry(nickname).map(|e| e.delete_credential());
}
