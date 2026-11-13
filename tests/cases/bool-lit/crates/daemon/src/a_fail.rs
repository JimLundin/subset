enum Live {
    On,
    Off,
}

struct Flags {
    live: Live,
}

impl Flags {
    fn on(&self) -> Live {
        self.live(true)
    }
}

fn off() -> Live {
    Live::from_flag(false)
}
