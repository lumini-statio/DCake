mod auth;
mod cache;
mod db;
mod domain;
mod join;
mod model;
mod outbox;
mod repositories;
mod services;
mod storage;
#[cfg(test)]
mod tests;

use std::cell::{Cell, RefCell};
use std::collections::VecDeque;
use std::rc::Rc;
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::Duration;

use slint::{Model, ModelRc, SharedString, VecModel};

use domain::services::{Service, message_service::MessageService, userxgroup_service::UserXGroupService};
use sea_orm::prelude::Uuid;
use model::{msg, new_group, participant};
use repositories::{group_repository::PgGroupRepository, message_repository::SqliteMessageRepository, user_repository::PgUserRepository};
use services::{group_service::GroupServiceImpl, message_service::MessageServiceImpl, userxgroup_service::UserXGroupServiceImpl};

slint::include_modules!();

fn main() -> Result<(), slint::PlatformError> {
    let ui = AppWindow::new()?;

    let rt = tokio::runtime::Runtime::new().expect("no se pudo crear el runtime de tokio");
    // Sin base la app igual abre; el error se muestra al intentar unirse a un grupo.
    let db = rt.block_on(db::connect()).inspect_err(|e| eprintln!("Sin conexión a la base de datos: {e}")).ok();
    let (rt, db) = (Rc::new(rt), Rc::new(db));
    // Usuario logueado (id de Supabase Auth): lo setea sign_in; lo usa join_group.
    let user_id = Rc::new(Cell::new(None));
    const DATA: &str = "data.json";

    // Deja `s` como la única sesión: guarda su token en el keyring y borra el de las demás cuentas.
    let sign_in = {
        let u = user_id.clone();
        Rc::new(move |s: auth::Session| -> SharedString {
            u.set(Some(s.user_id));
            let others = storage::load_accounts(DATA.as_ref()).unwrap_or_default();
            others.iter().filter(|n| **n != s.nickname).for_each(|n| auth::delete_token(n));
            auth::save_token(&s.nickname, &s.refresh_token).and_then(|_| storage::add_account(DATA.as_ref(), &s.nickname)).err().unwrap_or_default().into()
        })
    };

    // Panel de cuentas: si data.json tiene cuentas, se elige una antes del login.
    let accounts = storage::load_accounts(DATA.as_ref()).inspect_err(|e| eprintln!("{e}")).unwrap_or_default();
    ui.set_choosing_account(!accounts.is_empty());
    // Inicio automático: si una cuenta tiene token en el keyring y Supabase lo acepta, se entra sin mostrar ningún panel.
    // shortcut: bloquea el arranque mientras refresca el token, igual que login.
    if let Some((nick, token)) = accounts.iter().find_map(|n| Some((n, auth::load_token(n)?))) {
        match rt.block_on(auth::refresh(&token)) {
            Ok(Some(s)) => {
                let e = sign_in(s);
                if !e.is_empty() {
                    eprintln!("{e}");
                }
                ui.set_logged_in(true);
            }
            Ok(None) => {
                let e = session_expired(&ui, nick);
                ui.set_auth_error(e);
            }
            // Sin conexión no se sabe si venció: queda en el panel de cuentas para reintentar.
            Err(e) => eprintln!("No se pudo restaurar la sesión: {e}"),
        }
    }
    ui.set_accounts(ModelRc::new(VecModel::from(
        accounts.iter().map(|n| Account { nickname: n.as_str().into(), signed: auth::load_token(n).is_some() }).collect::<Vec<_>>(),
    )));

    let (r, si, w) = (rt.clone(), sign_in.clone(), ui.as_weak());
    ui.on_enter_account(move |nick| {
        let Some(token) = auth::load_token(&nick) else { return "Esa cuenta no tiene la sesión iniciada.".into() };
        match r.block_on(auth::refresh(&token)) {
            Ok(Some(s)) => si(s),
            Ok(None) => session_expired(&w.unwrap(), &nick),
            Err(e) => e.into(),
        }
    });

    let (r, si) = (rt.clone(), sign_in.clone());
    ui.on_login(move |email: SharedString, pass| {
        let (email, pass) = (email.trim(), pass.as_str());
        if email.is_empty() || pass.is_empty() {
            return "Ingresá email y contraseña.".into();
        }
        // shortcut: bloquea la UI mientras consulta, igual que join_group.
        r.block_on(auth::login(email, pass)).map_or_else(SharedString::from, |s| si(s))
    });

    let (r, si) = (rt.clone(), sign_in.clone());
    ui.on_register(move |name, nick, email, pass| {
        let (name, nick, email) = (name.trim(), nick.trim(), email.trim());
        if nick.is_empty() || email.is_empty() || pass.is_empty() {
            return "Nickname, email y contraseña son obligatorios.".into();
        }
        if nick.chars().count() > 32 {
            return "El nickname admite hasta 32 caracteres.".into();
        }
        if !email.contains('@') {
            return "Email inválido.".into();
        }
        match r.block_on(auth::register(nick, Some(name).filter(|n| !n.is_empty()), email, &pass)) {
            Ok(Some(s)) => si(s),
            Ok(None) => "Te mandamos un email para confirmar la cuenta; después iniciá sesión.".into(),
            Err(e) => e.into(),
        }
    });

    let groups = Rc::new(VecModel::from(model::demo_groups()));
    ui.set_groups(ModelRc::from(groups.clone()));

    let messages = rt.block_on(db::connect_sqlite("sqlite://messages.db?mode=rwc")).inspect_err(|e| eprintln!("Sin base de mensajes: {e}")).ok();
    let message_service = messages.map(|db| Arc::new(MessageServiceImpl(SqliteMessageRepository(db))));
    // Mensajes enviados (y, cuando existan, recibidos) que esperan el próximo guardado.
    let pending = outbox::Pending::default();

    let (g, u, p) = (groups.clone(), user_id.clone(), pending.clone());
    ui.on_send(move |group, text| {
        let text = text.trim();
        if text.is_empty() {
            return;
        }
        if let Some(group) = g.row_data(group as usize) {
            messages_of(&group).push(msg("Yo", text));
            // Los grupos de ejemplo no tienen id: sus mensajes no se guardan.
            if let (Ok(group_id), Some(user_id)) = (Uuid::parse_str(&group.id), u.get()) {
                p.lock().unwrap().push(domain::models::Message { id: Uuid::new_v4(), group_id, user_id, text: text.into() });
            }
        }
    });

    // Cada 5 minutos, los pendientes se guardan en un hilo aparte; si el anterior sigue guardando, se espera al próximo tick.
    let flushing: Rc<RefCell<Option<JoinHandle<()>>>> = Rc::default();
    let save_timer = slint::Timer::default();
    if let Some(service) = message_service.clone() {
        let (h, p, f) = (rt.handle().clone(), pending.clone(), flushing.clone());
        save_timer.start(slint::TimerMode::Repeated, Duration::from_secs(5 * 60), move || {
            let mut f = f.borrow_mut();
            if f.as_ref().is_some_and(|t| !t.is_finished()) || p.lock().unwrap().is_empty() {
                return;
            }
            *f = Some(outbox::flush(h.clone(), service.clone(), p.clone()));
        });
    }

    // Al entrar a un chat se cargan sus últimos 50 mensajes; quedan en memoria los de los últimos 3 chats abiertos.
    let open_chats = RefCell::new(VecDeque::new());
    let (g, r, u, p, ms) = (groups.clone(), rt.clone(), user_id.clone(), pending.clone(), message_service.clone());
    ui.on_open_chat(move |i| {
        let Some(group) = g.row_data(i as usize) else { return };
        // Los grupos de ejemplo no tienen id: no hay mensajes en la base.
        let Ok(id) = Uuid::parse_str(&group.id) else { return };
        let mut open_chats = open_chats.borrow_mut();
        if !open_chats.contains(&id) {
            let Some(service) = ms.as_ref() else { return };
            // shortcut: bloquea la UI mientras consulta, igual que join_group.
            let loaded = match r.block_on(service.get_messages(id, 50, 0)) {
                Ok(m) => m,
                Err(e) => return eprintln!("No se pudieron cargar los mensajes: {e}"),
            };
            let me = u.get();
            // shortcut: los mensajes ajenos muestran el inicio del uuid del autor; buscar el nickname cuando haya usuarios en la UI.
            let from = |m: &domain::models::Message| if Some(m.user_id) == me { "Yo".into() } else { m.user_id.to_string()[..8].to_owned() };
            // Se suman los que todavía no se guardaron, sin repetir los que ya llegaron a la base.
            let mut all: Vec<_> = loaded.into_iter().rev().collect();
            let unsaved: Vec<_> = p.lock().unwrap().iter().filter(|m| m.group_id == id && !all.iter().any(|x| x.id == m.id)).cloned().collect();
            all.extend(unsaved);
            let all = &all[all.len().saturating_sub(50)..];
            messages_of(&group).set_vec(all.iter().map(|m| msg(&from(m), &m.text)).collect::<Vec<_>>());
        }
        if let Some(old) = cache::touch(&mut open_chats, id, 3) {
            let old = SharedString::from(old.to_string());
            g.iter().filter(|x| x.id == old).for_each(|x| messages_of(&x).set_vec(vec![]));
        }
    });

    let handle = rt.handle().clone();
    let group_service = (*db).clone().map(|db| GroupServiceImpl(PgGroupRepository(db)));
    let userxgroup_service = (*db).clone().map(|db| UserXGroupServiceImpl(PgUserRepository(db)));
    ui.on_join_group(move |code| {
        let ids: Vec<SharedString> = groups.iter().map(|g| g.id).collect();
        match join::validate_code(&code, &ids) {
            Ok(id) => {
                let (Some(group_service), Some(userxgroup_service)) = (group_service.as_ref(), userxgroup_service.as_ref()) else {
                    return "Sin conexión a la base de datos.".into();
                };
                let Some(user) = user_id.get() else { return "Iniciá sesión para unirte a un grupo.".into() };
                // shortcut: la consulta bloquea la UI (hasta 5 s si la base no responde); pasar a slint::spawn_local si molesta.
                let name = match rt.block_on(group_service.find_by_pk(&id)) {
                    Ok(Some(group)) => group.name,
                    Ok(None) => return "El grupo no existe.".into(),
                    Err(e) => return format!("Error de base de datos: {e}").into(),
                };
                if let Err(e) = rt.block_on(userxgroup_service.add_relation(user, id)) {
                    return format!("Error de base de datos: {e}").into();
                }
                if let Err(e) = storage::save_group(DATA.as_ref(), &name, &id.to_string()) {
                    return e.into();
                }
                const COLORS: [u32; 5] = [0x3ba55d, 0xed4245, 0x9b59b6, 0xfaa61a, 0x1abc9c];
                groups.push(new_group(&id.to_string(), &name, None, COLORS[ids.len() % COLORS.len()], vec![], vec![participant("Yo", 0x5865f2, true)]));
                SharedString::new()
            }
            Err(e) => e.into(),
        }
    });

    let result = ui.run();

    // Al cerrar: se espera el guardado en curso y se guarda el resto en lotes de 100, también en un hilo aparte.
    drop(save_timer);
    if let Some(t) = flushing.take() {
        let _ = t.join();
    }
    if let Some(service) = message_service {
        let _ = outbox::flush(handle, service, pending.clone()).join();
    }
    let lost = pending.lock().unwrap().len();
    if lost > 0 {
        eprintln!("Se perdieron {lost} mensajes que no se pudieron guardar.");
    }
    result
}

fn messages_of(group: &Group) -> &VecModel<Message> {
    group.messages.as_any().downcast_ref::<VecModel<Message>>().unwrap()
}

/// Sesión vencida: borra el token del keyring y manda al login. Devuelve el mensaje a mostrar.
fn session_expired(ui: &AppWindow, nickname: &str) -> SharedString {
    auth::delete_token(nickname);
    ui.set_choosing_account(false);
    "Tu sesión venció. Iniciá sesión de nuevo.".into()
}
