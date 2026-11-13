#[derive(Debug, thiserror::Error)]
enum LoadError {
    #[error("reading {0}")]
    Read(String),
}
