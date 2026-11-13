impl Handle {
    pub async fn head(&self) -> Result<HostSeq, Stopped> {
        self.ask(|reply| Msg::Head { reply }).await
    }

    async fn ask<T>(&self, build: impl FnOnce(oneshot::Sender<T>) -> Msg) -> Result<T, Stopped> {
        let (reply, answer) = oneshot::channel();
        self.commands.send(build(reply)).await.map_err(|_| Stopped {})?;
        answer.await.map_err(|_| Stopped {})
    }
}

async fn stop(commands: &mpsc::Sender<Msg>) -> Result<Summary, Stopped> {
    ask(commands, |reply| Msg::Stop { reply }).await
}

async fn ask<T>(
    commands: &mpsc::Sender<Msg>,
    build: impl FnOnce(oneshot::Sender<T>) -> Msg,
) -> Result<T, Stopped> {
    let (reply, answer) = oneshot::channel();
    commands.send(build(reply)).await.map_err(|_| Stopped {})?;
    answer.await.map_err(|_| Stopped {})
}
