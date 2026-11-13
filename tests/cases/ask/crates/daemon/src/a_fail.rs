impl Handle {
    pub async fn ask<T>(&self, build: impl FnOnce(oneshot::Sender<T>) -> Msg) -> Result<T, Stopped> {
        self.send(build).await
    }
}

async fn request<T: Clone>(build: impl FnOnce(Sender<T>) -> Msg) -> T {
    send(build).await
}
