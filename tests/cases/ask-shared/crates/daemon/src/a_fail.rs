mod public {
    pub async fn ask<M, T>(inbox: &mpsc::Sender<M>, build: impl FnOnce(oneshot::Sender<T>) -> M) -> Result<T, Stopped> { answer }
}

mod restricted {
    pub(super) async fn ask<M, T>(inbox: &mpsc::Sender<M>, build: impl FnOnce(oneshot::Sender<T>) -> M) -> Result<T, Stopped> { answer }
}

impl Handle {
    async fn ask<M, T>(&self, build: impl FnOnce(oneshot::Sender<T>) -> M) -> Result<T, Stopped> { answer }
}

impl OtherHandle {
    pub(crate) async fn ask<M, T>(inbox: &mpsc::Sender<M>, build: impl FnOnce(oneshot::Sender<T>) -> M) -> Result<T, Stopped> { answer }
}

mod single {
    pub(crate) async fn ask<T>(inbox: &mpsc::Sender<Msg>, build: impl FnOnce(oneshot::Sender<T>) -> Msg) -> Result<T, Stopped> { answer }
}

mod bounded_message {
    async fn ask<M: Clone, T>(inbox: &mpsc::Sender<M>, build: impl FnOnce(oneshot::Sender<T>) -> M) -> Result<T, Stopped> { answer }
}

mod bounded_reply {
    async fn ask<M, T: Clone>(inbox: &mpsc::Sender<M>, build: impl FnOnce(oneshot::Sender<T>) -> M) -> Result<T, Stopped> { answer }
}

mod where_clause {
    async fn ask<M, T>(inbox: &mpsc::Sender<M>, build: impl FnOnce(oneshot::Sender<T>) -> M) -> Result<T, Stopped> where M: Clone { answer }
}

mod extra_generic {
    async fn ask<M, T, U>(inbox: &mpsc::Sender<M>, build: impl FnOnce(oneshot::Sender<T>) -> M) -> Result<T, Stopped> { answer }
}

mod owned_sender {
    async fn ask<M, T>(inbox: mpsc::Sender<M>, build: impl FnOnce(oneshot::Sender<T>) -> M) -> Result<T, Stopped> { answer }
}

mod mutable_sender {
    async fn ask<M, T>(inbox: &mut mpsc::Sender<M>, build: impl FnOnce(oneshot::Sender<T>) -> M) -> Result<T, Stopped> { answer }
}

mod wrong_sender {
    async fn ask<M, T>(inbox: &oneshot::Sender<M>, build: impl FnOnce(oneshot::Sender<T>) -> M) -> Result<T, Stopped> { answer }
}

mod wrong_item {
    async fn ask<M, T>(inbox: &mpsc::Sender<T>, build: impl FnOnce(oneshot::Sender<T>) -> M) -> Result<T, Stopped> { answer }
}

mod concrete_output {
    async fn ask<M, T>(inbox: &mpsc::Sender<M>, build: impl FnOnce(oneshot::Sender<T>) -> Msg) -> Result<T, Stopped> { answer }
}

mod wrapped_output {
    async fn ask<M, T>(inbox: &mpsc::Sender<M>, build: impl FnOnce(oneshot::Sender<T>) -> Option<M>) -> Result<T, Stopped> { answer }
}

mod wrong_reply {
    async fn ask<M, T>(inbox: &mpsc::Sender<M>, build: impl FnOnce(oneshot::Sender<M>) -> M) -> Result<T, Stopped> { answer }
}

mod wrong_channel {
    async fn ask<M, T>(inbox: &mpsc::Sender<M>, build: impl FnOnce(mpsc::Sender<T>) -> M) -> Result<T, Stopped> { answer }
}

mod extra_bound {
    async fn ask<M, T>(inbox: &mpsc::Sender<M>, build: impl FnOnce(oneshot::Sender<T>) -> M + Send) -> Result<T, Stopped> { answer }
}

mod wrong_helper {
    async fn request<M, T>(inbox: &mpsc::Sender<M>, build: impl FnOnce(oneshot::Sender<T>) -> M) -> Result<T, Stopped> { answer }
}

mod wrong_builder {
    async fn ask<M, T>(inbox: &mpsc::Sender<M>, make: impl FnOnce(oneshot::Sender<T>) -> M) -> Result<T, Stopped> { answer }
}

mod extra_parameter {
    async fn ask<M, T>(inbox: &mpsc::Sender<M>, value: Value, build: impl FnOnce(oneshot::Sender<T>) -> M) -> Result<T, Stopped> { answer }
}

mod message_as_error {
    async fn ask<M, T>(inbox: &mpsc::Sender<M>, build: impl FnOnce(oneshot::Sender<T>) -> M) -> Result<T, M> { answer }
}

mod qualified_output {
    async fn ask<M, T>(inbox: &mpsc::Sender<M>, build: impl FnOnce(oneshot::Sender<T>) -> <Other as Trait>::M) -> Result<T, Stopped> { answer }
}

mod qualified_message {
    async fn ask<M, T>(inbox: &mpsc::Sender<<Other as Trait>::M>, build: impl FnOnce(oneshot::Sender<T>) -> M) -> Result<T, Stopped> { answer }
}

mod qualified_reply {
    async fn ask<M, T>(inbox: &mpsc::Sender<M>, build: impl FnOnce(oneshot::Sender<<Other as Trait>::T>) -> M) -> Result<T, Stopped> { answer }
}
