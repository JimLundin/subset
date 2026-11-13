impl Drop for Guard {
    fn drop(&mut self) {
        let _ = self.child.start_kill();
    }
}
