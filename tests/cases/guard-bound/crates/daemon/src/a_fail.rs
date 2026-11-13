fn pick(x: Item, a: Seq, b: Seq) -> Label {
    match x {
        _ if a == b => Label::Same,
        _ => Label::Other,
    }
}

fn smuggle(v: Option<Item>, w: Item) -> Label {
    match v {
        Some(v) if matches!(v.is_empty() && w.is_empty(), true) => Label::Same,
        Some(_) => Label::Other,
        None => Label::None,
    }
}

fn inner(v: Option<Item>, w: Item) -> Label {
    match v {
        Some(v) if matches!(v.kind(), Kind::A if w.is_empty() || !v.is_empty()) => Label::Same,
        Some(_) | None => Label::Other,
    }
}

fn write(item: Option<Item>) -> Label {
    match item {
        Some(Item::Request { kind, .. }) if matches!(kind, Kind::Write) => Label::Write,
        Some(_) | None => Label::Other,
    }
}

fn first(seq: Option<Seq>) -> Label {
    match seq {
        Some(seq) if seq == Seq(0) => Label::First,
        Some(_) | None => Label::Other,
    }
}

fn live(events: &[Event], b: Seq) -> usize {
    events
        .iter()
        .filter(|x| matches!(x.seq == b && x.live(), true))
        .count()
}
