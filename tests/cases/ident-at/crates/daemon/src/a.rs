pub enum Event {
    Open { id: u64 },
    Closed,
}

pub fn opened(event: Event) -> Option<Event> {
    match event {
        open @ Event::Open { .. } => Some(open),
        Event::Closed => None,
    }
}
