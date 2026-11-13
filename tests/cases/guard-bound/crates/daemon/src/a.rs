fn write(item: Option<Item>) -> Label {
    match item {
        Some(item) if matches!(item.kind(), Kind::Write) => Label::Write,
        Some(_) | None => Label::Other,
    }
}

fn changed(current: Status, next: Status) -> Change {
    match current {
        current if current != next => Change::Yes,
        _ => Change::No,
    }
}

fn paired(events: &[Event]) -> usize {
    events
        .iter()
        .filter(|e| matches!((e.kind(), e.seq()), (Kind::Done, Some(_))))
        .count()
}
