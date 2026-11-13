#[derive(Debug, thiserror::Error)]
enum LoadError {
    #[error(transparent)]
    Io(std::io::Error),
    Parse(String),
}
