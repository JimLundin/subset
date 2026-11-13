enum Visibility { Visible, Hidden }
fn application_flag(flag: Option<bool>) -> Visibility {
    match flag { Some(true) => Visibility::Hidden, Some(false) | None => Visibility::Visible }
}
#[derive(Deserialize)]
struct OutsideWire { #[serde(default)] hidden: bool }
