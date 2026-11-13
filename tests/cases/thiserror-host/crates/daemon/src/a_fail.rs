#[derive(Debug, thiserror::Error)]
enum Outcome {
    #[error("done")]
    Done,
}
