use std::ops::{Bound, Range, RangeFull, RangeInclusive};

fn iterated(bytes: &[u8]) -> Head {
    Head::read(bytes.get(RangeInclusive::new(1, 2).into_iter()))
}

fn bounds(bytes: &[u8]) -> Head {
    let low = Bound::Included(1);
    let high = Bound::Excluded(3);
    Head::read(bytes.get((low, high)))
}

fn open(bytes: &[u8]) -> Head {
    let high = Bound::Unbounded;
    Head::read(bytes.get((Bound::Included(1), high)))
}

fn full(bytes: &[u8]) -> Head {
    Head::read(bytes.get(RangeFull::default()))
}

fn typed(bytes: &[u8]) -> Head {
    Head::read(bytes.get(Range::<usize>::default()))
}

fn split(bytes: &[u8]) -> Head {
    Head::read(bytes.split_at(2))
}
