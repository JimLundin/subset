#[derive(Debug, thiserror::Error)]
enum LoadError {
    #[error("reading: {0}")]
    Read(#[from] std::io::Error),
    #[error("parsing: {0}")]
    Parse(#[from] toml::de::Error),
    #[error("decoding: {0}")]
    Decode(#[from] DecodeError),
}

#[derive(Debug, thiserror::Error)]
enum DecodeError {
    #[error("empty line")]
    Empty,
}
