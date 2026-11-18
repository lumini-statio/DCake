use serde_json::{Value, json};

/// Lee `path`, o `{}` si no existe.
fn read(path: &std::path::Path) -> Result<Value, String> {
    match std::fs::read_to_string(path) {
        // Si el archivo existe pero está corrupto, se falla en vez de pisarlo y perder datos.
        Ok(s) => serde_json::from_str(&s).map_err(|e| format!("data.json inválido: {e}")),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(json!({})),
        Err(e) => Err(e.to_string()),
    }
}

/// Lee `path` (o `{}` si no existe), aplica `f` sobre el objeto raíz y lo vuelve a escribir.
fn edit(path: &std::path::Path, f: impl FnOnce(&mut serde_json::Map<String, Value>) -> Result<(), String>) -> Result<(), String> {
    let mut data = read(path)?;
    f(data.as_object_mut().ok_or("data.json inválido: no es un objeto")?)?;
    std::fs::write(path, serde_json::to_string_pretty(&data).unwrap()).map_err(|e| e.to_string())
}

/// Agrega el grupo a la clave "groups" de `path`, creando el archivo si no existe.
pub fn save_group(path: &std::path::Path, name: &str, id: &str) -> Result<(), String> {
    edit(path, |data| {
        let groups = data
        .entry("groups")
        .or_insert_with(|| json!([]))
        .as_array_mut()
        .ok_or("data.json inválido: \"groups\" no es una lista")?;
        groups.push(json!({ "name": name, "id": id }));
        Ok(())
    })
}

/// Agrega `nickname` a "cuentas" si no estaba. Solo se guarda el nickname: el token va al keyring.
pub fn add_account(path: &std::path::Path, nickname: &str) -> Result<(), String> {
    edit(path, |data| {
        // Perfil de versiones anteriores (nickname, email): ya no se guarda.
        data.remove("profile");
        let cuentas = data.entry("cuentas").or_insert_with(|| json!([])).as_array_mut().ok_or("data.json inválido: \"cuentas\" no es una lista")?;
        // Entradas de versiones anteriores (con id y signed) quedan solo con el nickname.
        *cuentas = cuentas.iter().filter_map(|c| c["nickname"].as_str()).map(|n| json!({ "nickname": n })).collect();
        if !cuentas.iter().any(|c| c["nickname"] == nickname) {
            cuentas.push(json!({ "nickname": nickname }));
        }
        Ok(())
    })
}

/// Nicknames guardados en "cuentas"; ignora entradas mal formadas.
pub fn load_accounts(path: &std::path::Path) -> Result<Vec<String>, String> {
    let data = read(path)?;
    let Some(cuentas) = data.get("cuentas").and_then(Value::as_array) else { return Ok(vec![]) };
    Ok(cuentas.iter().filter_map(|c| Some(c["nickname"].as_str()?.to_owned())).collect())
}
