use slint::{Color, Image, ModelRc, Rgb8Pixel, SharedPixelBuffer, SharedString, VecModel};

use crate::{Group, Message, Participant};

pub fn initial(name: &str) -> SharedString {
    name.chars().next().map(|c| c.to_uppercase().collect::<String>()).unwrap_or_default().into()
}

pub fn participant(name: &str, color: u32, cam: bool) -> Participant {
    Participant { name: name.into(), initial: initial(name), color: rgb(color), cam }
}

pub fn msg(from: &str, text: &str) -> Message {
    Message { from: from.into(), text: text.into(), mine: from == "Yo" }
}

// Ícono de ejemplo generado en memoria, para mostrar un grupo con imagen.
fn demo_icon() -> Image {
    let mut buf = SharedPixelBuffer::<Rgb8Pixel>::new(64, 64);
    for (i, px) in buf.make_mut_slice().iter_mut().enumerate() {
        let (x, y) = ((i % 64) as u8, (i / 64) as u8);
        *px = Rgb8Pixel { r: x * 4, g: 120, b: y * 4 };
    }
    Image::from_rgb8(buf)
}

fn rgb(c: u32) -> Color {
    Color::from_rgb_u8((c >> 16) as u8, (c >> 8) as u8, c as u8)
}

pub fn new_group(id: &str, name: &str, icon: Option<Image>, color: u32, msgs: Vec<Message>, parts: Vec<Participant>) -> Group {
    Group {
        id: id.into(),
        name: name.into(),
        initial: initial(name),
        icon: icon.unwrap_or_default(),
        color: rgb(color),
        messages: ModelRc::new(VecModel::from(msgs)),
        participants: ModelRc::new(VecModel::from(parts)),
    }
}

// Los grupos de ejemplo no tienen id: no existen en la base.
pub fn demo_groups() -> Vec<Group> {
    vec![
        new_group("", "Familia", Some(demo_icon()), 0x3ba55d, vec![msg("Mamá", "¿Vienen el domingo?"), msg("Yo", "¡Sí, llevamos postre!")],
            vec![participant("Yo", 0x5865f2, true), participant("Mamá", 0xeb459e, true), participant("Papá", 0xfaa61a, false)]),
        new_group("", "Trabajo", None, 0xed4245, vec![msg("Ana", "La reunión pasa a las 15hs"), msg("Luis", "Ok")],
            vec![participant("Yo", 0x5865f2, true), participant("Ana", 0x3ba55d, true), participant("Luis", 0xfaa61a, false),
                 participant("Sofía", 0xeb459e, true), participant("Diego", 0x9b59b6, false)]),
        new_group("", "amigos", None, 0x9b59b6, vec![msg("Juan", "¿Partido hoy?")],
            vec![participant("Yo", 0x5865f2, false), participant("Juan", 0x1abc9c, true)]),
    ]
}

#[test]
fn initial_works() {
    assert_eq!(initial("amigos"), "A");
    assert_eq!(initial("ñandú"), "Ñ");
    assert_eq!(initial(""), "");
}
