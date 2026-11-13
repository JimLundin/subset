fn drain(mut rx: Inbox) {
    let _ = rx.try_recv();
}
