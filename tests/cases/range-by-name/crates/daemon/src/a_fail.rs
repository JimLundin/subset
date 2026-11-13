use std::ops::{Bound, Range, RangeFull, RangeInclusive};

fn header(bytes: &[u8]) -> Head {
    Head::read(bytes.get(RangeInclusive::new(1, 2)))
}

fn full(bytes: &[u8]) -> Head {
    Head::read(bytes.get(std::ops::RangeInclusive::new(1, 2)))
}

fn body(bytes: &[u8]) -> Head {
    Head::read(bytes.get(Range { start: 1, end: 2 }))
}

fn all(bytes: &[u8]) -> Head {
    Head::read(bytes.get(RangeFull))
}

fn bounded(bytes: &[u8]) -> Head {
    Head::read(bytes.get((Bound::Included(1), Bound::Excluded(3))))
}

fn kept(first: u16, last: u16) -> Ports {
    let range = RangeInclusive::new(first, last);
    Ports { range }
}

fn spelled(bytes: &[u8]) -> Head {
    Head::read(bytes.get(1..=2))
}
