fn mutable_tail(bytes: &[u8]) -> Vec<u8> {
    match bytes {
        [mut remaining @ ..] => remaining.to_vec(),
    }
}

fn borrowed_tail(bytes: &[u8]) -> Vec<u8> {
    match bytes {
        [ref remaining @ ..] => remaining.to_vec(),
    }
}

fn boolean_element(value: External) -> External {
    match value {
        [flag @ true, ..] => flag,
        _ => external(),
    }
}

fn narrowed_length(bytes: &[u8], marker: u8) -> Option<Vec<u8>> {
    match bytes {
        [first, ..] if first == &marker => Some(bytes.to_vec()),
        [_, _] => None,
        [] => None,
    }
}

fn narrowed_prefix(bytes: &[u8], marker: u8) -> Option<Vec<u8>> {
    match bytes {
        [first, ..] if first == &marker => Some(bytes.to_vec()),
        [0x01, ..] => None,
        [] => None,
    }
}

fn narrowed_suffix(bytes: &[u8], marker: u8) -> Option<Vec<u8>> {
    match bytes {
        [first, .., last] if first == &marker => Some(Vec::from([last.to_owned()])),
        [_, .., 0x02] => None,
        [] => None,
        [_] => None,
    }
}
