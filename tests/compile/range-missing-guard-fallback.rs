fn guarded(value: u8, expected: u8) -> Option<u8> {
    match value {
        n @ 1..=3 if n == expected => Some(n),
        ..1 => None,
        4.. => None,
    }
}
