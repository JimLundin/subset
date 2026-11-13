fn on_open(tx: Sender) -> Callback {
    Closure::new(move |event| {
        let _ = tx.send(Msg::Open(event));
    })
}
