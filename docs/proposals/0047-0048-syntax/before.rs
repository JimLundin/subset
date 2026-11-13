enum Event {
    Open,
    Updated,
    Closed,
}

fn joined(first: String, second: String) -> String {
    Vec::from([first, second]).concat()
}

fn mapped() -> Vec<String> {
    Vec::from(["a", "b"]).into_iter().map(String::from).collect()
}

fn selected(event: Event) -> Option<Event> {
    match event {
        event @ Event::Open => Some(event),
        event @ Event::Updated => Some(event),
        Event::Closed => None,
    }
}

fn payload(bytes: &[u8]) -> Option<Vec<u8>> {
    match bytes.split_first() {
        Some((0x01, remaining)) => Some(remaining.to_vec()),
        _ => None,
    }
}
