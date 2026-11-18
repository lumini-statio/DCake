use sea_orm::prelude::Uuid;
use slint::SharedString;

/// Valida el código ingresado (el uuid del grupo) contra los ids de los grupos ya unidos.
pub fn validate_code(code: &str, existing: &[SharedString]) -> Result<Uuid, &'static str> {
    let code = code.trim();
    if code.is_empty() {
        return Err("Ingresá un código.");
    }
    let id = Uuid::parse_str(code).map_err(|_| "El código no es válido.")?;
    if existing.iter().any(|n| Uuid::parse_str(n).ok() == Some(id)) {
        return Err("Ya estás en ese grupo.");
    }
    Ok(id)
}
