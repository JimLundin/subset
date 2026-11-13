fn signed(value: i16) -> Option<i16> {
    match value {
        0 => None,
        n @ 1..=3 => Some(n),
        4.. => None,
    }
}
