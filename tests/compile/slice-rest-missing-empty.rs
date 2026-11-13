fn prefix(bytes: &[u8]) -> Option<Vec<u8>> {
    match bytes {
        [first, remaining @ ..] => Some(remaining.to_vec()),
    }
}
