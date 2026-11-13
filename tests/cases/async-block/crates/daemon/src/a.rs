fn start(rx: Inbox) {
    let _ = tokio::spawn(async move { work(rx).await });
}

fn events(rx: Inbox) -> Events {
    stream::unfold(rx, |rx| async move { next(rx).await })
}
