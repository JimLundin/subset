enum AttemptState {
    Fresh,
    Retrying,
    Exhausted,
}

fn classify(attempts: u8) -> AttemptState {
    match attempts {
        0 => AttemptState::Fresh,
        _ => match std::ops::RangeInclusive::new(1, 3)
            .contains(&attempts)
            .then_some(AttemptState::Retrying)
        {
            Some(state) => state,
            None => AttemptState::Exhausted,
        },
    }
}

fn letter(value: char) -> Option<char> {
    value.is_ascii_alphabetic().then_some(value)
}
