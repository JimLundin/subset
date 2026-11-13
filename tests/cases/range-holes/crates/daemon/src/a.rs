use std::ops::RangeInclusive;

fn inside(first: u16, last: u16, port: u16) -> Inside {
    Inside::from(RangeInclusive::new(first, last).contains(&port))
}

fn listed(first: u16, last: u16) -> Vec<u16> {
    RangeInclusive::new(first, last).into_iter().collect()
}

fn head(bytes: &[u8]) -> Head {
    Head::read(bytes.split_at_checked(2))
}
