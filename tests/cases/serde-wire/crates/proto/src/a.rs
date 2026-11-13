use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Reply {
    pub is_error: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub interrupt: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Message {
    Response(Reply),
    Failure { is_error: bool },
}

pub fn ok() -> Reply {
    Reply {
        is_error: false,
        interrupt: None,
    }
}
