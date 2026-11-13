enum Event {
    Open,
    Updated,
    Closed,
}

fn joined(first: String, second: String) -> String {
    [first, second].concat()
}

fn mapped() -> Vec<String> {
    ["a", "b"].map(String::from).to_vec()
}

fn selected(event: Event) -> Option<Event> {
    match event {
        event @ (Event::Open | Event::Updated) => Some(event),
        Event::Closed => None,
    }
}

fn payload(bytes: &[u8]) -> Option<Vec<u8>> {
    match bytes {
        [0x01, remaining @ ..] => Some(remaining.to_vec()),
        _ => None,
    }
}
