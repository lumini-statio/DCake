use crate::model::initial;

#[test]
fn initial_works() {
    assert_eq!(initial("amigos"), "A");
    assert_eq!(initial("ñandú"), "Ñ");
    assert_eq!(initial(""), "");
}
