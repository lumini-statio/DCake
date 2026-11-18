use std::collections::VecDeque;

/// Chats con mensajes en memoria, del más reciente (adelante) al más viejo (atrás).
/// Pone `id` adelante; si la cola pasa de `cap`, saca y devuelve el último.
pub fn touch<T: PartialEq>(order: &mut VecDeque<T>, id: T, cap: usize) -> Option<T> {
    order.retain(|x| *x != id);
    order.push_front(id);
    if order.len() > cap { order.pop_back() } else { None }
}
