use slint::{Color, Image, ModelRc, SharedString, VecModel};

use crate::{Group, Message, User};

pub fn initial(name: &str) -> SharedString {
    name.chars().next().map(|c| c.to_uppercase().collect::<String>()).unwrap_or_default().into()
}

pub fn participant(name: &str, color: u32, cam: bool) -> User {
    User { name: name.into(), initial: initial(name), color: rgb(color), cam }
}

pub fn msg(from: &str, text: &str) -> Message {
    Message { from: from.into(), text: text.into(), mine: from == "Yo" }
}

fn rgb(c: u32) -> Color {
    Color::from_rgb_u8((c >> 16) as u8, (c >> 8) as u8, c as u8)
}

pub fn new_group(id: &str, name: &str, icon: Option<Image>, color: u32, msgs: Vec<Message>, parts: Vec<User>) -> Group {
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
