fn admit(open: Option<Open>, token: AuthToken, host: HostName) -> Result<Hello, Refusal> {
    match open {
        Some(Open::Hello(hello)) if hello.token == token && hello.host == host => Ok(hello),
        Some(Open::Hello(hello)) if !hello.anonymous() => Err(Refusal::WrongToken),
        Some(Open::Hello(hello)) if hello.signed() => Err(Refusal::WrongToken),
        Some(Open::Hello(_)) => Err(Refusal::WrongToken),
        Some(Open::Subscribe(_)) | None => Err(Refusal::NoHello),
    }
}

fn reply(hello: Option<Hello>, token: AuthToken, mode: Mode) -> Reply {
    match (hello, mode) {
        (Some(hello), _) if hello.token == token => Reply::Welcome,
        (Some(_), Mode::Strict) => Reply::Refuse,
        (Some(_), Mode::Open) => Reply::Guest,
        (None, _) => Reply::Refuse,
    }
}
