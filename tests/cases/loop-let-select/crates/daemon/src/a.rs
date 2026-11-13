impl Log {
    async fn run(mut self, mut inbox: Inbox) -> Result<Infallible, Stopped> {
        loop {
            let input = tokio::select! {
                msg = inbox.recv() => Input::Msg(msg),
                tick = self.ticks.next() => Input::Tick(tick),
            };
            self.on(input).await?;
        }
    }
}
