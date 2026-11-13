async fn drive(inbox: Inbox) -> Outcome {
    loop {
        match inbox.recv().await {
            Some(msg) => handle(msg),
            None => break,
        }
    }
}
