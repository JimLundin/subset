impl Log {
    async fn run(mut self, mut inbox: Inbox) -> Result<Infallible, Stopped> {
        loop {
            let input = Input::from(tokio::select! {
                msg = inbox.recv() => msg,
            });
            self.on(input).await?;
        }
    }
}
