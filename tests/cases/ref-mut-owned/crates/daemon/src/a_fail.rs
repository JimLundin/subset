struct Session {
    turn: Turn,
}

impl Session {
    fn finish(&mut self, outcome: Outcome) {
        self.turn.finish(outcome);
    }
}

fn apply(turn: &mut Turn, outcome: Outcome) {
    turn.finish(outcome);
}

struct Conn {}

impl Conn {
    fn close(&mut self) {}
}
