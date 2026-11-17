use slint::SharedString;

/// Valida el código ingresado contra los ids de los grupos ya unidos; devuelve el código limpio o el mensaje de error.
pub fn validate_code<'a>(code: &'a str, existing: &[SharedString]) -> Result<&'a str, &'static str> {
    let code = code.trim();
    if code.is_empty() {
        return Err("Ingresá un código.");
    }
    if code.chars().count() > 24 || !code.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_') {
        return Err("Solo letras, números, - o _ (máx. 24).");
    }
    if existing.iter().any(|n| n.to_lowercase() == code.to_lowercase()) {
        return Err("Ya estás en ese grupo.");
    }
    Ok(code)
}

#[test]
fn validate_code_works() {
    let existing = [SharedString::from("Trabajo")];
    assert_eq!(validate_code("  abc-1 ", &existing), Ok("abc-1"));
    assert!(validate_code("", &existing).is_err());
    assert!(validate_code("con espacio", &existing).is_err());
    assert!(validate_code("trabajo", &existing).is_err());
}
