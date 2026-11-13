fn float(value: f64) -> Option<f64> {
    match value {
        n @ 1.0..=2.0 => Some(n),
        _ => None,
    }
}

fn boolean(value: External) -> External {
    match value {
        flag @ true..=false => flag,
        _ => external(),
    }
}

fn string(value: External) -> External {
    match value {
        text @ "a"..="z" => text,
        _ => external(),
    }
}

fn exclusive_end(value: u8, expected: u8) -> Option<u8> {
    match value {
        n @ 1..=3 if n == expected => Some(n),
        1..3 => None,
        0 => None,
        4.. => None,
    }
}

fn narrower_start(value: u8, expected: u8) -> Option<u8> {
    match value {
        n @ 1..=3 if n == expected => Some(n),
        2..=4 => None,
        ..1 => None,
        5.. => None,
    }
}

fn symbolic(value: u8, expected: u8) -> Option<u8> {
    match value {
        n @ limits::LOW..=limits::HIGH if n == expected => Some(n),
        limits::LOWER..=limits::HIGHER => None,
        5.. => None,
    }
}

fn mutable(value: u8) -> Option<u8> {
    match value {
        mut n @ 1..=3 => Some(n),
        _ => None,
    }
}

fn referenced(value: u8) -> External {
    match value {
        ref n @ 1..=3 => n,
        _ => external(),
    }
}

fn range_value(value: u8) -> External {
    let range = 1..=3;
    use_range(range, value)
}

fn narrowed_slice(bytes: &[u8], expected: u8) -> Option<Vec<u8>> {
    match bytes {
        [n @ 1..=3, remaining @ ..] if n == &expected => Some(remaining.to_vec()),
        [1..3, ..] => None,
        [] => None,
        [(4..), ..] => None,
    }
}
