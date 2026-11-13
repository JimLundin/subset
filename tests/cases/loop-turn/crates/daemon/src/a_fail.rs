async fn run(inbox: Inbox) -> Result<Infallible, Stopped> {
    loop {
        let msg = inbox.recv().await.ok_or(Stopped {})?;
        log(&msg);
        handle(msg);
    }
}
