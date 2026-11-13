async fn run(inbox: Inbox) {
    let host = Host::new();
    loop {
        host.on(inbox.recv().await);
    }
}
