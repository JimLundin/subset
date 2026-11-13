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

fn either(event: Event) -> Event {
    match event {
        event @ (Event::Open { .. } | Event::Closed) => event,
    }
}

fn number(value: u64) -> Option<u64> {
    match value {
        number @ (1 | 2 | 3) => Some(number),
        _ => None,
    }
}

fn literal(value: u64) -> Option<u64> {
    match value {
        number @ 3 => Some(number),
        _ => None,
    }
}

fn range(value: u64) -> Option<u64> {
    match value {
        number @ 1..=3 => Some(number),
        _ => None,
    }
}

fn pair(values: (u64, u64)) -> (u64, u64) {
    match values {
        pair @ (_, _) => pair,
    }
}

fn destructured_pair(values: (u64, u64)) -> u64 {
    match values {
        pair @ (a, b) => pair.0.saturating_add(a).saturating_add(b),
    }
}

fn wildcard(value: String) -> String {
    match value {
        whole @ _ => whole,
    }
}

fn nested(event: Option<Event>) -> Option<Event> {
    match event {
        Some(event @ (Event::Open { .. } | Event::Closed)) => Some(event),
        None => None,
    }
}

fn alias(value: u64) -> (u64, u64) {
    match value {
        whole @ part => (whole, part),
    }
}

fn grouped(event: Event) -> Event {
    match event {
        (event @ ((Event::Open { .. } | Event::Closed))) => event,
    }
}
