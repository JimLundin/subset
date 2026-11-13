use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct Batch {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub items: Vec<String>,
}

pub struct Session {
    pub live: bool,
}

pub fn start() -> Session {
    Session { live: true }
}
