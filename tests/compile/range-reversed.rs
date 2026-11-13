fn reversed(value: u8) -> Option<u8> {
    match value {
        n @ 3..=1 => Some(n),
        _ => None,
    }
}
