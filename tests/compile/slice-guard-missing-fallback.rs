fn guarded(bytes: &[u8], marker: u8) -> Option<Vec<u8>> {
    match bytes {
        [first, remaining @ ..] if first == &marker => Some(remaining.to_vec()),
        [] => None,
    }
}
