enum AttemptState {
    Fresh,
    Retrying,
    Exhausted,
}

fn classify(attempts: u8) -> AttemptState {
    match attempts {
        0 => AttemptState::Fresh,
        1..=3 => AttemptState::Retrying,
        4.. => AttemptState::Exhausted,
    }
}

fn letter(value: char) -> Option<char> {
    match value {
        letter @ ('a'..='z' | 'A'..='Z') => Some(letter),
        _ => None,
    }
}
