pub fn small(n: u64) -> Option<u64> {
    match n {
        x @ 3 => Some(x),
        _ => None,
    }
}
