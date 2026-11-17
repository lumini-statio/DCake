/// Lee `path` (o `{}` si no existe), aplica `f` sobre el objeto raíz y lo vuelve a escribir.
fn edit(path: &std::path::Path, f: impl FnOnce(&mut serde_json::Map<String, serde_json::Value>) -> Result<(), String>) -> Result<(), String> {
    let mut data = match std::fs::read_to_string(path) {
        // Si el archivo existe pero está corrupto, se falla en vez de pisarlo y perder datos.
        Ok(s) => serde_json::from_str(&s).map_err(|e| format!("data.json inválido: {e}"))?,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => serde_json::json!({}),
        Err(e) => return Err(e.to_string()),
    };
    f(data.as_object_mut().ok_or("data.json inválido: no es un objeto")?)?;
    std::fs::write(path, serde_json::to_string_pretty(&data).unwrap()).map_err(|e| e.to_string())
}

/// Guarda el perfil (sin contraseña) en la clave "profile" de `path`, reemplazando el anterior.
pub fn save_profile(path: &std::path::Path, p: &crate::auth::Profile) -> Result<(), String> {
    edit(path, |data| {
        data.insert("profile".into(), serde_json::json!({ "name": p.name, "nickname": p.nickname, "email": p.email }));
        Ok(())
    })
}

/// Agrega el grupo a la clave "groups" de `path`, creando el archivo si no existe.
pub fn save_group(path: &std::path::Path, name: &str, id: &str) -> Result<(), String> {
    edit(path, |data| {
        let groups = data
        .entry("groups")
        .or_insert_with(|| serde_json::json!([]))
        .as_array_mut()
        .ok_or("data.json inválido: \"groups\" no es una lista")?;
        groups.push(serde_json::json!({ "name": name, "id": id }));
        Ok(())
    })
}

#[test]
fn save_group_works() {
    let path = std::env::temp_dir().join("ominium-save_group.json");
    let _ = std::fs::remove_file(&path);
    save_group(&path, "a", "c1").unwrap();
    save_group(&path, "b", "c2").unwrap();
    let v: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    assert_eq!(v["groups"], serde_json::json!([{"name": "a", "id": "c1"}, {"name": "b", "id": "c2"}]));
    save_profile(&path, &crate::auth::Profile { name: None, nickname: "ana".into(), email: Some("a@b.c".into()) }).unwrap();
    let v: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    assert_eq!(v["profile"], serde_json::json!({"name": null, "nickname": "ana", "email": "a@b.c"}));
    assert_eq!(v["groups"].as_array().unwrap().len(), 2);
    std::fs::write(&path, "{roto").unwrap();
    assert!(save_group(&path, "x", "y").is_err());
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "{roto");
    std::fs::remove_file(&path).unwrap();
}
