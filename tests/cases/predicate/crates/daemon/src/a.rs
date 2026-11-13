fn finished(events: &[Event]) -> usize {
    events.iter().filter(|e| matches!(e.kind, Kind::TurnFinished)).count()
}

fn seen(events: &[Event], seq: HostSeq) -> Option<&Event> {
    events.iter().find(|e| e.seq == seq)
}

fn first_live(events: &[Event], seq: HostSeq) -> Option<usize> {
    events.iter().position(|e| e.seq >= seq && !matches!(e.kind, Kind::Replayed))
}
