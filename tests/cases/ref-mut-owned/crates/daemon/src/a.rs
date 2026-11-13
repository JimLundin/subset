struct Reader {
    frames: FrameBuffer,
    turns: HashMap<ThreadId, Turn>,
    phase: Phase,
}

enum Phase {
    Idle,
    Live(Live),
}

struct Live {
    conn: Conn,
}

impl Live {
    fn arm(&mut self) {
        self.conn = Conn {};
    }
}

struct FrameBuffer {
    pending: Vec<u8>,
}

struct Turn {
    outcome: Option<Outcome>,
}

impl Reader {
    async fn run(mut self, mut inbox: Inbox) -> Result<Infallible, Stopped> {
        loop {
            let msg = inbox.recv().await.ok_or(Stopped {})?;
            self.on(msg);
        }
    }

    fn on(&mut self, msg: Msg) {
        self.frames.push(msg.bytes);
    }
}

impl FrameBuffer {
    fn push(&mut self, bytes: Vec<u8>) {
        self.pending.extend(bytes);
    }
}

impl Turn {
    fn finish(&mut self, outcome: Outcome) {
        self.outcome = Some(outcome);
    }
}
