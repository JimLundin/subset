#[derive(Debug, thiserror::Error)]
enum LoadError {
    #[error("reading: {0}")]
    Read(#[from] std::io::Error),
    #[error("writing: {0}")]
    Write(#[from] std::io::Error),
}
