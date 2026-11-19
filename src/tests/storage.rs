use crate::storage::{add_account, load_accounts, load_groups, save_group};

#[test]
fn save_group_works() {
    let path = std::env::temp_dir().join("ominium-save_group.json");
    let _ = std::fs::remove_file(&path);
    save_group(&path, "ana", "a", "c1").unwrap();
    save_group(&path, "ana", "b", "c2").unwrap();
    save_group(&path, "luis", "c", "c3").unwrap();
    add_account(&path, "ana").unwrap();
    assert_eq!(load_groups(&path, "ana").unwrap(), [("a".into(), "c1".into()), ("b".into(), "c2".into())], "add_account no pisa los grupos");
    assert_eq!(load_groups(&path, "luis").unwrap(), [("c".to_owned(), "c3".to_owned())], "cada cuenta ve solo sus grupos");
    assert!(load_groups(&path, "otro").unwrap().is_empty());
    std::fs::write(&path, "{roto").unwrap();
    assert!(save_group(&path, "ana", "x", "y").is_err());
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

#[test]
fn load_groups_sin_archivo_o_con_entradas_rotas() {
    let path = std::env::temp_dir().join("jawyo-load_groups.json");
    let _ = std::fs::remove_file(&path);
    assert!(load_groups(&path, "ana").unwrap().is_empty());
    std::fs::write(&path, r#"{"cuentas": [{"nickname": "ana", "groups": [{"name": "a"}, {"name": "b", "id": "c2"}]}]}"#).unwrap();
    assert_eq!(load_groups(&path, "ana").unwrap(), [("b".to_owned(), "c2".to_owned())]);
    std::fs::remove_file(&path).unwrap();
}

#[test]
fn grupos_sin_cuenta_pasan_a_la_primera_que_entra() {
    let path = std::env::temp_dir().join("jawyo-migrar-grupos.json");
    std::fs::write(&path, r#"{"cuentas": [{"nickname": "ana"}], "groups": [{"name": "a", "id": "c1"}]}"#).unwrap();
    add_account(&path, "ana").unwrap();
    add_account(&path, "luis").unwrap();
    assert_eq!(load_groups(&path, "ana").unwrap(), [("a".to_owned(), "c1".to_owned())]);
    assert!(load_groups(&path, "luis").unwrap().is_empty());
    let v: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    assert!(v.get("groups").is_none());
    std::fs::remove_file(&path).unwrap();
}
