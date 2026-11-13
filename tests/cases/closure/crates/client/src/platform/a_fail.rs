fn on_open(tx: Sender) -> Callback {
    Callback::wrap(move |event| {
        let _ = tx.send(Msg::Open(event));
    })
}
