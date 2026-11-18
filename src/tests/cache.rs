use std::collections::VecDeque;

use crate::cache::touch;

#[test]
fn touch_desaloja_el_ultimo() {
    let mut q = VecDeque::new();
    assert_eq!(touch(&mut q, 1, 3), None);
    assert_eq!(touch(&mut q, 2, 3), None);
    assert_eq!(touch(&mut q, 3, 3), None);
    // Reabrir un chat cargado lo pasa adelante sin desalojar.
    assert_eq!(touch(&mut q, 1, 3), None);
    assert_eq!(q, [1, 3, 2]);
    assert_eq!(touch(&mut q, 4, 3), Some(2));
    assert_eq!(q, [4, 1, 3]);
}
