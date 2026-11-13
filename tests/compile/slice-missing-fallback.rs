fn exact(bytes: &[u8]) -> Option<Vec<u8>> {
    match bytes {
        [first, second] => Some(Vec::from([first.to_owned(), second.to_owned()])),
    }
}
