pub(crate) async fn ask<M, T>(
    inbox: &mpsc::Sender<M>,
    build: impl FnOnce(oneshot::Sender<T>) -> M,
) -> Result<T, Stopped> {
    let (reply, answer) = oneshot::channel();
    inbox.send(build(reply)).await.map_err(|_| Stopped {})?;
    answer.await.map_err(|_| Stopped {})
}

impl Log {
    pub async fn head(&self) -> Result<HostSeq, Stopped> {
        ask(&self.inbox, |reply| LogMsg::Head { reply }).await
    }
}

impl Registry {
    pub async fn find(&self, session: SessionId) -> Result<Option<Session>, Stopped> {
        crate::ask(&self.inbox, |reply| RegistryMsg::Find { session, reply }).await
    }
}

mod private {
    async fn ask<Message, Reply>(
        inbox: &tokio::sync::mpsc::Sender<Message>,
        build: impl FnOnce(tokio::sync::oneshot::Sender<Reply>) -> Message,
    ) -> Result<Reply, Stopped> {
        let (reply, answer) = oneshot::channel();
        inbox.send(build(reply)).await.map_err(|_| Stopped {})?;
        answer.await.map_err(|_| Stopped {})
    }
}
