impl Log {
    fn with_head(mut self, head: HostSeq) -> Log {
        self.head = head;
        self
    }
}
