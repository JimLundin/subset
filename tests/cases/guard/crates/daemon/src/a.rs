fn admit(open: Option<Open>, token: AuthToken) -> Result<Hello, Refusal> {
    match open {
        Some(Open::Hello(hello)) if hello.token == token => Ok(hello),
        Some(Open::Hello(_)) => Err(Refusal::WrongToken),
        Some(Open::Subscribe(_)) | None => Err(Refusal::NoHello),
    }
}

fn label(item: Item, change: &Change) -> Label {
    match (item, change) {
        (Item::Tool { call, .. }, Change::Finish(id)) if &call == id => Label::Done,
        (request, _) if matches!(request.kind(), Kind::Write) => Label::Write,
        (item, _) => item.label(),
    }
}

fn feed(welcome: Welcome, subscribe: Subscribe) -> Reply {
    match welcome {
        Welcome { epoch, .. } if epoch == subscribe.epoch => Reply::Replay,
        Welcome { .. } => Reply::Close,
    }
}
