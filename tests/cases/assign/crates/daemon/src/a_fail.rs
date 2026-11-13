impl Log {
    fn head(&self, seen: Seen) {
        seen.seq = self.head;
    }
}
