fn admit(hello: Hello, token: AuthToken) -> Option<Hello> {
    Some(hello).filter(|hello| hello.token == token)
}

fn finished(call: CallId, id: &CallId, outcome: Outcome) -> Option<Outcome> {
    call.eq(id).then_some(outcome)
}

fn changed(status: &Status, next: &Status) -> Option<Change> {
    status.ne(next).then_some(Change::Status)
}
