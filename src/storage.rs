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

/// Agrega el grupo a los "groups" de la cuenta `nickname` en `path`, creando el archivo y la cuenta si no existen.
pub fn save_group(path: &std::path::Path, nickname: &str, name: &str, id: &str) -> Result<(), String> {
    edit(path, |data| {
        let cuenta = account_mut(data, nickname)?;
        let groups = cuenta.entry("groups").or_insert_with(|| json!([])).as_array_mut().ok_or("data.json inválido: \"groups\" no es una lista")?;
        groups.push(json!({ "name": name, "id": id }));
        Ok(())
    })
}

/// Entrada de `nickname` en "cuentas"; la crea si no está.
fn account_mut<'a>(data: &'a mut serde_json::Map<String, Value>, nickname: &str) -> Result<&'a mut serde_json::Map<String, Value>, String> {
    let cuentas = data.entry("cuentas").or_insert_with(|| json!([])).as_array_mut().ok_or("data.json inválido: \"cuentas\" no es una lista")?;
    let i = match cuentas.iter().position(|c| c["nickname"] == nickname) {
        Some(i) => i,
        None => {
            cuentas.push(json!({ "nickname": nickname }));
            cuentas.len() - 1
        }
    };
    cuentas[i].as_object_mut().ok_or_else(|| "data.json inválido: cuenta mal formada".into())
}

/// Agrega `nickname` a "cuentas" si no estaba. Se guarda el nickname y sus grupos: el token va al keyring.
pub fn add_account(path: &std::path::Path, nickname: &str) -> Result<(), String> {
    edit(path, |data| {
        // Perfil de versiones anteriores (nickname, email): ya no se guarda.
        data.remove("profile");
        // Grupos de versiones anteriores, sin cuenta: pasan a la primera cuenta que inicie sesión.
        let old_groups = data.remove("groups");
        let cuentas = data.entry("cuentas").or_insert_with(|| json!([])).as_array_mut().ok_or("data.json inválido: \"cuentas\" no es una lista")?;
        // Entradas de versiones anteriores (con id y signed) quedan solo con el nickname y sus grupos.
        *cuentas = cuentas
            .iter()
            .filter_map(|c| {
                let mut e = json!({ "nickname": c["nickname"].as_str()? });
                if c["groups"].is_array() {
                    e["groups"] = c["groups"].clone();
                }
                Some(e)
            })
            .collect();
        let cuenta = account_mut(data, nickname)?;
        if let Some(Value::Array(old)) = old_groups {
            let groups = cuenta.entry("groups").or_insert_with(|| json!([])).as_array_mut().ok_or("data.json inválido: \"groups\" no es una lista")?;
            groups.splice(0..0, old);
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

/// Grupos de la cuenta `nickname` como (nombre, id), en el orden del archivo (del más viejo al más nuevo); ignora entradas mal formadas.
pub fn load_groups(path: &std::path::Path, nickname: &str) -> Result<Vec<(String, String)>, String> {
    let data = read(path)?;
    let cuenta = data.get("cuentas").and_then(Value::as_array).and_then(|c| c.iter().find(|c| c["nickname"] == nickname));
    let Some(groups) = cuenta.and_then(|c| c["groups"].as_array()) else { return Ok(vec![]) };
    Ok(groups.iter().filter_map(|g| Some((g["name"].as_str()?.to_owned(), g["id"].as_str()?.to_owned()))).collect())
}
