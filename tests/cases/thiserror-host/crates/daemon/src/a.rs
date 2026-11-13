#[derive(Debug, thiserror::Error)]
enum Lost {
    #[error("the peer closed")]
    Closed,
}

fn wait() -> Result<u8, Lost> {
    Err(Lost::Closed)
}
