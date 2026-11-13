pub struct Counter {
    pub seen: u64,
}

impl Counter {
    fn bump(&mut self) {
        self.seen = self.seen.saturating_add(1);
    }
}
