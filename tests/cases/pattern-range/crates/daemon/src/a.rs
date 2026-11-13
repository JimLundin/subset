enum AttemptState {
    Fresh,
    Retrying,
    Exhausted,
}

fn unsigned(attempts: u8) -> AttemptState {
    match attempts {
        0 => AttemptState::Fresh,
        1..=3 => AttemptState::Retrying,
        4.. => AttemptState::Exhausted,
    }
}

fn signed(value: i16) -> Option<i16> {
    match value {
        ..=-1 => None,
        n @ 0..=3 => Some(n),
        4.. => None,
    }
}

fn exclusive(value: u8) -> Option<u8> {
    match value {
        ..3 => None,
        n @ 3..4 => Some(n),
        4.. => None,
    }
}

fn character(value: char) -> Option<char> {
    match value {
        letter @ ('a'..='z' | 'A'..='Z') => Some(letter),
        _ => None,
    }
}

fn byte(value: u8) -> Option<u8> {
    match value {
        letter @ b'a'..=b'z' => Some(letter),
        _ => None,
    }
}

struct Retry {
    attempt: u8,
}

fn nested(retry: Retry, expected: u8) -> Option<u8> {
    match retry {
        Retry { attempt: n @ (1..=3) } if n == expected => Some(n),
        Retry { attempt: 0..=4 } => None,
        _ => None,
    }
}

fn tuple(value: (Option<i16>, char)) -> Option<i16> {
    match value {
        (Some(n @ (-3..=3)), 'a'..='z') => Some(n),
        _ => None,
    }
}

fn slice(bytes: &[u8]) -> Option<Vec<u8>> {
    match bytes {
        packet @ ([0x01..=0x03, ..] | [0x08..=0x09, ..]) => Some(packet.to_vec()),
        _ => None,
    }
}

mod bounds {
    pub const LOW: u8 = 1;
    pub const HIGH: u8 = 3;
}

fn constants(value: u8, expected: u8) -> Option<u8> {
    match value {
        n @ bounds::LOW..=bounds::HIGH if n == expected => Some(n),
        bounds::LOW..=bounds::HIGH => None,
        _ => None,
    }
}

fn widened(value: i16, expected: i16) -> Option<i16> {
    match value {
        n @ -3..=3 if n == expected => Some(n),
        -4..4 => None,
        _ => None,
    }
}

fn open_lower(value: i16, expected: i16) -> Option<i16> {
    match value {
        n @ -3..=3 if n == expected => Some(n),
        ..=4 => None,
        _ => None,
    }
}

fn open_upper(value: i16, expected: i16) -> Option<i16> {
    match value {
        n @ -3..=3 if n == expected => Some(n),
        -4.. => None,
        _ => None,
    }
}

fn widened_character(value: char, expected: char) -> Option<char> {
    match value {
        letter @ 'b'..='x' if letter == expected => Some(letter),
        'a'..'z' => None,
        _ => None,
    }
}

fn literal_fallback(value: u8, expected: u8) -> Option<u8> {
    match value {
        n @ 3 if n == expected => Some(n),
        1..=3 => None,
        _ => None,
    }
}

fn inclusive_fallback(value: u8, expected: u8) -> Option<u8> {
    match value {
        n @ 1..3 if n == expected => Some(n),
        1..=3 => None,
        _ => None,
    }
}

fn open_guard(value: i16, expected: i16) -> Option<i16> {
    match value {
        n @ ..3 if n == expected => Some(n),
        ..4 => None,
        4.. => None,
    }
}

fn slice_guard(bytes: &[u8], expected: u8) -> Option<Vec<u8>> {
    match bytes {
        [n @ 1..=3, remaining @ ..] if n == &expected => Some(remaining.to_vec()),
        [0..=4, ..] => None,
        _ => None,
    }
}

fn full_binding(value: u8) -> u8 {
    let n @ 0..=255 = value;
    n
}

fn full_parameter(n @ 0..=255: u8) -> u8 {
    n
}

fn one_case(value: u8) {
    if let n @ 1..=3 = value {
        consume(n);
    }
}

fn consume(_value: u8) {}

fn wide_unsigned(value: u128, expected: u128) -> Option<u128> {
    match value {
        n @ 170141183460469231731687303715884105728..=340282366920938463463374607431768211455 if n == expected => Some(n),
        170141183460469231731687303715884105728..=340282366920938463463374607431768211455 => None,
        _ => None,
    }
}
