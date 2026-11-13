impl Log {
    async fn run(mut self, mut inbox: Inbox) -> Result<Infallible, Stopped> {
        loop {
            let msg = inbox.recv().await.ok_or(Stopped {})?;
            self.on(msg);
        }
    }

    fn on(&mut self, msg: Msg) {
        self.head = msg.seq;
    }
}

async fn drive(inbox: Inbox) -> Outcome {
    let mut state = Turns::new();
    loop {
        match inbox.recv().await {
            Some(Msg::Done(outcome)) => break outcome,
            Some(msg) => state = state.on(msg),
            None => break Outcome::Lost,
        }
    }
}

impl Turns {
    fn count(&mut self) {
        self.seen = self.seen.saturating_add(1);
    }
}
