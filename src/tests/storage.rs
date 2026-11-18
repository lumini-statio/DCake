use crate::storage::{add_account, load_accounts, save_group};

#[test]
fn save_group_works() {
    let path = std::env::temp_dir().join("ominium-save_group.json");
    let _ = std::fs::remove_file(&path);
    save_group(&path, "a", "c1").unwrap();
    save_group(&path, "b", "c2").unwrap();
    let v: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    assert_eq!(v["groups"], serde_json::json!([{"name": "a", "id": "c1"}, {"name": "b", "id": "c2"}]));
    add_account(&path, "ana").unwrap();
    let v: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    assert_eq!(v["groups"].as_array().unwrap().len(), 2, "add_account no pisa los grupos");
    std::fs::write(&path, "{roto").unwrap();
    assert!(save_group(&path, "x", "y").is_err());
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "{roto");
    std::fs::remove_file(&path).unwrap();
}

#[test]
fn cuentas_solo_guardan_el_nickname() {
    let path = std::env::temp_dir().join("jawyo-cuentas.json");
    // Formato viejo: perfil con email y cuentas con id/signed.
    std::fs::write(&path, r#"{"profile": {"email": "a@b.c"}, "cuentas": [{"nickname": "ana", "id": "x", "signed": true}]}"#).unwrap();

    add_account(&path, "luis").unwrap();
    add_account(&path, "ana").unwrap();

    assert_eq!(load_accounts(&path).unwrap(), ["ana", "luis"], "sin duplicados y orden estable");
    let v: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    assert_eq!(v, serde_json::json!({"cuentas": [{"nickname": "ana"}, {"nickname": "luis"}]}), "solo nicknames, sin perfil");
    std::fs::remove_file(&path).unwrap();
}
