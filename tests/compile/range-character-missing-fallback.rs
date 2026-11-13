fn character(value: char) -> Option<char> {
    match value {
        letter @ ('a'..='z' | 'A'..='Z') => Some(letter),
    }
}
