use std::ops::RangeInclusive;

fn candidates(first: u16, last: u16) -> Vec<u16> {
    RangeInclusive::new(first, last).collect()
}

fn numbered(names: Vec<Name>) -> Vec<Numbered> {
    RangeInclusive::new(1, 100).zip(names).map(Numbered::from).collect()
}

fn named(names: Vec<Name>) -> Vec<Numbered> {
    names.into_iter().zip(RangeInclusive::new(1, 100)).map(Numbered::from).collect()
}

fn bind(sockets: &Sockets, first: u16, last: u16) {
    for port in RangeInclusive::new(first, last) {
        sockets.bind(port);
    }
}
