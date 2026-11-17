mod auth;
mod db;
mod join;
mod model;
mod storage;

use std::rc::Rc;

use slint::{Model, ModelRc, SharedString, VecModel};

use model::{msg, new_group, participant};

slint::include_modules!();

fn main() -> Result<(), slint::PlatformError> {
    let ui = AppWindow::new()?;

    let rt = tokio::runtime::Runtime::new().expect("no se pudo crear el runtime de tokio");
    // Sin base la app igual abre; el error se muestra al intentar unirse a un grupo.
    let db = rt.block_on(db::connect()).inspect_err(|e| eprintln!("Sin conexión a la base de datos: {e}")).ok();
    let (rt, db) = (Rc::new(rt), Rc::new(db));

    let (r, d) = (rt.clone(), db.clone());
    ui.on_login(move |nick: SharedString, pass| {
        let (nick, pass) = (nick.trim(), pass.as_str());
        if nick.is_empty() || pass.is_empty() {
            return "Ingresá nickname y contraseña.".into();
        }
        let Some(db) = d.as_ref() else { return "Sin conexión a la base de datos.".into() };
        // shortcut: bloquea la UI mientras consulta, igual que join_group.
        match r.block_on(auth::login(db, nick, pass)) {
            Ok(Some(p)) => storage::save_profile("data.json".as_ref(), &p).err().unwrap_or_default().into(),
            Ok(None) => "Nickname o contraseña incorrectos.".into(),
            Err(e) => format!("Error de base de datos: {e}").into(),
        }
    });

    let (r, d) = (rt.clone(), db.clone());
    ui.on_register(move |name, nick, email, pass| {
        let opt = |s: &str| Some(s.trim().to_owned()).filter(|s| !s.is_empty());
        let p = auth::Profile { name: opt(&name), nickname: nick.trim().to_owned(), email: opt(&email) };
        if p.nickname.is_empty() || pass.is_empty() {
            return "Nickname y contraseña son obligatorios.".into();
        }
        if p.nickname.chars().count() > 32 {
            return "El nickname admite hasta 32 caracteres.".into();
        }
        if p.email.as_ref().is_some_and(|e| !e.contains('@')) {
            return "Email inválido.".into();
        }
        let Some(db) = d.as_ref() else { return "Sin conexión a la base de datos.".into() };
        match r.block_on(auth::register(db, &p, &pass)) {
            Ok(true) => storage::save_profile("data.json".as_ref(), &p).err().unwrap_or_default().into(),
            Ok(false) => "Ese nickname ya está en uso.".into(),
            Err(e) => format!("Error de base de datos: {e}").into(),
        }
    });

    let groups = Rc::new(VecModel::from(model::demo_groups()));
    ui.set_groups(ModelRc::from(groups.clone()));

    let g = groups.clone();
    ui.on_send(move |group, text| {
        let text = text.trim();
        if text.is_empty() {
            return;
        }
        if let Some(group) = g.row_data(group as usize) {
            let messages = group.messages.as_any().downcast_ref::<VecModel<Message>>().unwrap();
            messages.push(msg("Yo", text));
        }
    });

    ui.on_join_group(move |code| {
        let ids: Vec<SharedString> = groups.iter().map(|g| g.id).collect();
        match join::validate_code(&code, &ids) {
            Ok(code) => {
                let Some(db) = db.as_ref() else {
                    return "Sin conexión a la base de datos.".into();
                };
                // shortcut: la consulta bloquea la UI (hasta 5 s si la base no responde); pasar a slint::spawn_local si molesta.
                let name = match rt.block_on(db::find_group_name(db, code)) {
                    Ok(Some(name)) => name,
                    Ok(None) => return "El grupo no existe.".into(),
                    Err(e) => return format!("Error de base de datos: {e}").into(),
                };
                if let Err(e) = storage::save_group("data.json".as_ref(), &name, code) {
                    return e.into();
                }
                const COLORS: [u32; 5] = [0x3ba55d, 0xed4245, 0x9b59b6, 0xfaa61a, 0x1abc9c];
                groups.push(new_group(code, &name, None, COLORS[ids.len() % COLORS.len()], vec![], vec![participant("Yo", 0x5865f2, true)]));
                SharedString::new()
            }
            Err(e) => e.into(),
        }
    });

    ui.run()
}
