fn mutable(value: u64) -> Option<u64> {
    match value {
        mut number @ 3 => Some(number),
        _ => None,
    }
}

fn borrowed(value: u64) -> Option<External> {
    match value {
        ref number @ 3 => Some(number),
        _ => None,
    }
}

fn boolean(value: External) -> External {
    match value {
        flag @ (true | false) => flag,
    }
}
