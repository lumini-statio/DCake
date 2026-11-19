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
    // Usuario logueado (id de Supabase Auth) y su nickname: los setea sign_in; los usa join_group.
    let user_id = Rc::new(Cell::new(None));
    let nickname = Rc::new(RefCell::new(String::new()));
    const DATA: &str = "data.json";

    // Chats como pila: el más nuevo arriba (índice 0). Se cargan de data.json al iniciar sesión.
    let groups = Rc::new(VecModel::<Group>::default());
    ui.set_groups(ModelRc::from(groups.clone()));

    // Deja `s` como la única sesión: guarda su token en el keyring y borra el de las demás cuentas.
    let sign_in = {
        let (u, n, g) = (user_id.clone(), nickname.clone(), groups.clone());
        Rc::new(move |s: auth::Session| -> SharedString {
            u.set(Some(s.user_id));
            let others = storage::load_accounts(DATA.as_ref()).unwrap_or_default();
            others.iter().filter(|n| **n != s.nickname).for_each(|n| auth::delete_token(n));
            let err = auth::save_token(&s.nickname, &s.refresh_token).and_then(|_| storage::add_account(DATA.as_ref(), &s.nickname)).err().unwrap_or_default();
            let saved = storage::load_groups(DATA.as_ref(), &s.nickname).inspect_err(|e| eprintln!("{e}")).unwrap_or_default();
            // El color sale de la posición desde el más viejo, igual que al unirse, para que no cambie entre aperturas.
            let mut all: Vec<_> = saved.iter().enumerate().map(|(i, (name, id))| new_group(id, name, None, COLORS[i % COLORS.len()], vec![], vec![participant("Yo", 0x5865f2, true)])).collect();
            all.reverse();
            g.set_vec(all);
            *n.borrow_mut() = s.nickname;
            err.into()
        })
    };

    // Panel de cuentas: si data.json tiene cuentas, se elige una antes del login.
    let accounts = storage::load_accounts(DATA.as_ref()).inspect_err(|e| eprintln!("{e}")).unwrap_or_default();
    ui.set_choosing_account(!accounts.is_empty());
    // Inicio automático: si una cuenta tiene token en el keyring y Supabase lo acepta, se entra sin elegir cuenta.
    if let Some((nick, token)) = accounts.iter().find_map(|n| Some((n.clone(), auth::load_token(n)?))) {
        let si = sign_in.clone();
        run_busy(&ui, &rt, async move { auth::refresh(&token).await }, move |ui, r| match r {
            Ok(Some(s)) => {
                let e = si(s);
                if !e.is_empty() {
                    eprintln!("{e}");
                }
                ui.set_logged_in(true);
            }
            Ok(None) => ui.set_auth_error(session_expired(ui, &nick)),
            // Sin conexión no se sabe si venció: queda en el panel de cuentas para reintentar.
            Err(e) => eprintln!("No se pudo restaurar la sesión: {e}"),
        });
    }
    ui.set_accounts(ModelRc::new(VecModel::from(
        accounts.iter().map(|n| Account { nickname: n.as_str().into(), signed: auth::load_token(n).is_some() }).collect::<Vec<_>>(),
    )));

    // Muestra el resultado de un intento de entrada: "" entra, otro texto es el error.
    fn auth_result(ui: &AppWindow, e: SharedString) {
        ui.set_logged_in(e.is_empty());
        ui.set_auth_error(e);
    }

    let (r, si, w) = (rt.clone(), sign_in.clone(), ui.as_weak());
    ui.on_enter_account(move |nick| {
        let ui = w.unwrap();
        let Some(token) = auth::load_token(&nick) else { return auth_result(&ui, "Esa cuenta no tiene la sesión iniciada.".into()) };
        let si = si.clone();
        run_busy(&ui, &r, async move { auth::refresh(&token).await }, move |ui, res| {
            let e = match res {
                Ok(Some(s)) => si(s),
                Ok(None) => session_expired(ui, &nick),
                Err(e) => e.into(),
            };
            auth_result(ui, e);
        });
    });

    let (r, si, w) = (rt.clone(), sign_in.clone(), ui.as_weak());
    ui.on_login(move |email: SharedString, pass| {
        let ui = w.unwrap();
        let (email, pass) = (email.trim().to_owned(), pass.to_string());
        if email.is_empty() || pass.is_empty() {
            return auth_result(&ui, "Ingresá email y contraseña.".into());
        }
        let si = si.clone();
        run_busy(&ui, &r, async move { auth::login(&email, &pass).await }, move |ui, res| auth_result(ui, res.map_or_else(SharedString::from, |s| si(s))));
    });

    let (r, si, w) = (rt.clone(), sign_in.clone(), ui.as_weak());
    ui.on_register(move |name, nick, email, pass| {
        let ui = w.unwrap();
        let (name, nick, email, pass) = (name.trim().to_owned(), nick.trim().to_owned(), email.trim().to_owned(), pass.to_string());
        if nick.is_empty() || email.is_empty() || pass.is_empty() {
            return auth_result(&ui, "Nickname, email y contraseña son obligatorios.".into());
        }
        if nick.chars().count() > 32 {
            return auth_result(&ui, "El nickname admite hasta 32 caracteres.".into());
        }
        if !email.contains('@') {
            return auth_result(&ui, "Email inválido.".into());
        }
        let si = si.clone();
        let fut = async move { auth::register(&nick, Some(name.as_str()).filter(|n| !n.is_empty()), &email, &pass).await };
        run_busy(&ui, &r, fut, move |ui, res| {
            let e = match res {
                Ok(Some(s)) => si(s),
                Ok(None) => "Te mandamos un email para confirmar la cuenta; después iniciá sesión.".into(),
                Err(e) => e.into(),
            };
            auth_result(ui, e);
        });
    });

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
    let open_chats = Rc::new(RefCell::new(VecDeque::new()));
    // Marca `id` como el chat más reciente y vacía el que sale de la memoria.
    let touch = {
        let (g, oc) = (groups.clone(), open_chats.clone());
        move |id: Uuid| {
            if let Some(old) = cache::touch(&mut oc.borrow_mut(), id, 3) {
                let old = SharedString::from(old.to_string());
                g.iter().filter(|x| x.id == old).for_each(|x| messages_of(&x).set_vec(vec![]));
            }
        }
    };
    let (g, r, u, p, ms, w) = (groups.clone(), rt.clone(), user_id.clone(), pending.clone(), message_service.clone(), ui.as_weak());
    ui.on_open_chat(move |i| {
        let Some(group) = g.row_data(i as usize) else { return };
        // Los grupos de ejemplo no tienen id: no hay mensajes en la base.
        let Ok(id) = Uuid::parse_str(&group.id) else { return };
        if open_chats.borrow().contains(&id) {
            return touch(id);
        }
        let Some(service) = ms.clone() else { return };
        let (u, p, touch) = (u.clone(), p.clone(), touch.clone());
        run_busy(&w.unwrap(), &r, async move { service.get_messages(id, 50, 0).await }, move |_, res| {
            let loaded = match res {
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
            touch(id);
        });
    });

    let handle = rt.handle().clone();
    let group_service = (*db).clone().map(|db| Arc::new(GroupServiceImpl(PgGroupRepository(db))));
    let userxgroup_service = (*db).clone().map(|db| Arc::new(UserXGroupServiceImpl(PgUserRepository(db))));
    let w = ui.as_weak();
    ui.on_join_group(move |code| {
        let ui = w.unwrap();
        let ids: Vec<SharedString> = groups.iter().map(|g| g.id).collect();
        let id = match join::validate_code(&code, &ids) {
            Ok(id) => id,
            Err(e) => return ui.set_join_error(e.into()),
        };
        let (Some(gs), Some(uxg)) = (group_service.clone(), userxgroup_service.clone()) else {
            return ui.set_join_error("Sin conexión a la base de datos.".into());
        };
        let Some(user) = user_id.get() else { return ui.set_join_error("Iniciá sesión para unirte a un grupo.".into()) };
        let fut = async move {
            let name = match gs.find_by_pk(&id).await {
                Ok(Some(group)) => group.name,
                Ok(None) => return Err("El grupo no existe.".to_owned()),
                Err(e) => return Err(format!("Error de base de datos: {e}")),
            };
            uxg.add_relation(user, id).await.map_err(|e| format!("Error de base de datos: {e}"))?;
            Ok(name)
        };
        let (groups, nickname) = (groups.clone(), nickname.clone());
        run_busy(&ui, &rt, fut, move |ui, res| {
            let name = match res.and_then(|name| storage::save_group(DATA.as_ref(), &nickname.borrow(), &name, &id.to_string()).map(|_| name)) {
                Ok(name) => name,
                Err(e) => return ui.set_join_error(e.into()),
            };
            groups.insert(0, new_group(&id.to_string(), &name, None, COLORS[ids.len() % COLORS.len()], vec![], vec![participant("Yo", 0x5865f2, true)]));
            ui.invoke_joined();
        });
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

const COLORS: [u32; 5] = [0x3ba55d, 0xed4245, 0x9b59b6, 0xfaa61a, 0x1abc9c];

fn messages_of(group: &Group) -> &VecModel<Message> {
    group.messages.as_any().downcast_ref::<VecModel<Message>>().unwrap()
}

/// Corre `fut` en tokio sin bloquear la UI, mostrando la animación de carga (`busy`) hasta que termina;
/// `done` recibe el resultado en el hilo de la UI. Si ya hay otra operación en curso, no hace nada.
fn run_busy<T: Send + 'static>(ui: &AppWindow, rt: &tokio::runtime::Runtime, fut: impl Future<Output = T> + Send + 'static, done: impl FnOnce(&AppWindow, T) + 'static) {
    if ui.get_busy() {
        return;
    }
    ui.set_busy(true);
    let (task, w) = (rt.spawn(fut), ui.as_weak());
    let spawned = slint::spawn_local(async move {
        let res = task.await;
        let Some(ui) = w.upgrade() else { return };
        ui.set_busy(false);
        match res {
            Ok(v) => done(&ui, v),
            Err(e) => eprintln!("La operación falló: {e}"),
        }
    });
    if let Err(e) = spawned {
        eprintln!("No se pudo lanzar la operación: {e}");
        ui.set_busy(false);
    }
}

/// Sesión vencida: borra el token del keyring y manda al login. Devuelve el mensaje a mostrar.
fn session_expired(ui: &AppWindow, nickname: &str) -> SharedString {
    auth::delete_token(nickname);
    ui.set_choosing_account(false);
    "Tu sesión venció. Iniciá sesión de nuevo.".into()
}
