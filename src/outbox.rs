use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};

use tokio::runtime::Handle;

use crate::domain::models::Message;
use crate::domain::services::message_service::MessageService;

/// Mensajes enviados o recibidos que todavía no están en la base, en orden de llegada.
pub type Pending = Arc<Mutex<Vec<Message>>>;

const BATCH: usize = 100;

/// Guarda en lotes de 100 y frena en el primer error. Devuelve cuántos guardó (siempre un prefijo de `msgs`).
pub async fn save_in_batches<S: MessageService>(service: &S, msgs: &[Message]) -> usize {
    let mut saved = 0;
    for chunk in msgs.chunks(BATCH) {
        if let Err(e) = service.save_all(chunk).await {
            eprintln!("No se pudieron guardar mensajes: {e}");
            break;
        }
        saved += chunk.len();
    }
    saved
}

/// Guarda los pendientes en un hilo aparte que termina al terminar el guardado.
/// Los guardados salen de `pending`; los que fallan quedan para el próximo intento.
/// Debe haber un solo flush a la vez: solo él saca mensajes de `pending`.
pub fn flush<S: MessageService + Send + Sync + 'static>(rt: Handle, service: Arc<S>, pending: Pending) -> JoinHandle<()> {
    thread::spawn(move || {
        let snapshot = pending.lock().unwrap().clone();
        let saved = rt.block_on(save_in_batches(&*service, &snapshot));
        pending.lock().unwrap().drain(..saved);
    })
}
