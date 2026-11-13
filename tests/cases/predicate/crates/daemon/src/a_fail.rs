fn same(a: HostSeq, b: HostSeq) -> Same {
    let same = a == b;
    Same::from(same)
}

fn done(next: Option<Event>) {
    if let Some(event) = next && matches!(event.kind, Kind::TurnFinished) {
        finish(event);
    }
}

fn label(next: Option<Event>) -> Label {
    match next {
        Some(e) if matches!(e.kind, Kind::TurnFinished) => Label::Done,
        _ => Label::Busy,
    }
}

fn all(events: &[Event]) -> usize {
    events.iter().filter(|_| true).count()
}

fn counted(events: &[Event]) -> usize {
    events.iter().filter_map(|e| match e.kind {
        Kind::TurnFinished => Some(()),
        _ => None,
    }).count()
}
