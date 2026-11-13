use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct ModelRow {
    #[serde(default)]
    hidden: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    effort: Option<String>,
}
#[derive(Debug, PartialEq, Eq)]
pub enum Visibility { Visible, Hidden }
pub fn visibility(hidden: Option<bool>) -> Visibility {
    match hidden { Some(true) => Visibility::Hidden, Some(false) | None => Visibility::Visible }
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum Event {
    Open,
    #[serde(other)]
    Other,
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "kind", content = "payload")]
pub enum Adjacent {
    Known(String),
    #[serde(other)]
    Other,
}
#[derive(Debug, thiserror::Error)]
pub enum LoadError {
    #[error("loading {path}")]
    Io { path: String, #[source] cause: std::io::Error },
}
pub const PINS: &str = include_str!("pins.toml");

#[test]
fn missing_fields_default_but_null_stays_distinct() {
    let missing = serde_json::from_str::<ModelRow>("{}").unwrap();
    assert_eq!(visibility(Some(missing.hidden)), Visibility::Visible);
    assert_eq!(missing.effort, None);
    assert_eq!(serde_json::from_str::<ModelRow>("{\"hidden\":null}").err().map(|_| "invalid"), Some("invalid"));
    let null = serde_json::from_str::<ModelRow>("{\"effort\":null}").unwrap();
    assert_eq!(null.effort, None);
    assert_eq!(serde_json::to_value(&missing).unwrap(), serde_json::json!({"hidden":false}));
}
#[test]
fn unknown_tags_have_an_explicit_state() {
    let event = serde_json::from_str::<Event>("{\"kind\":\"Future\"}").unwrap();
    assert_eq!(format!("{event:?}"), "Other");
    let adjacent = serde_json::from_str::<Adjacent>("{\"kind\":\"Future\"}").unwrap();
    assert_eq!(format!("{adjacent:?}"), "Other");
}
#[test]
fn booleans_become_domain_states() {
    assert_eq!(visibility(Some(true)), Visibility::Hidden);
    assert_eq!(visibility(Some(false)), Visibility::Visible);
    assert_eq!(visibility(None), Visibility::Visible);
}
#[test]
fn source_retains_the_typed_cause() {
    use std::error::Error;
    let error = LoadError::Io { path: String::from("pins.toml"), cause: std::io::Error::other("disk unavailable") };
    assert_eq!(error.to_string(), "loading pins.toml");
    assert_eq!(error.source().map(ToString::to_string), Some(String::from("disk unavailable")));
}
#[test]
fn embedded_configuration_matches_the_file() {
    assert_eq!(PINS, "provider = \"codex\"\n");
}
