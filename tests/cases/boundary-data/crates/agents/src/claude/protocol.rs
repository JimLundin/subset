#[derive(Deserialize)]
#[serde(default)]
struct Container { field: String }
#[derive(Deserialize)]
struct CustomDefault { #[serde(default = "default_field")] field: String }
#[derive(Deserialize)]
enum UntaggedByDefault { Known, #[serde(other)] Other }
#[derive(Deserialize)]
#[serde(tag = "kind")]
enum PayloadFallback { Known, #[serde(other)] Other(String) }
struct NotWire { #[serde(default)] field: String }
struct NotError { #[source] cause: External }
#[derive(Debug, thiserror::Error)]
#[error("bad sources")]
struct BadError { #[source] first: External, #[source] second: External }
#[derive(Debug, thiserror::Error)]
#[error("bad attribute")]
struct AttributeError { #[source("wrong")] cause: External }
fn embedded() -> External { include_str!("pins.toml") }
const DYNAMIC: &str = include_str!(concat!("pins", ".toml"));
