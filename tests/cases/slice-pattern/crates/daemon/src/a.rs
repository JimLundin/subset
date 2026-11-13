fn exact(bytes: &[u8]) -> Option<Vec<u8>> {
    match bytes {
        [first, second] => Some(Vec::from([first.to_owned(), second.to_owned()])),
        _ => None,
    }
}

fn payload(bytes: &[u8]) -> Option<Vec<u8>> {
    match bytes {
        [0x01, remaining @ ..] => Some(remaining.to_vec()),
        _ => None,
    }
}

fn both_ends(bytes: &[u8]) -> Option<Vec<u8>> {
    match bytes {
        [first, middle @ .., last] => Some(
            std::iter::once(first.to_owned())
                .chain(middle.iter().copied())
                .chain(std::iter::once(last.to_owned()))
                .collect()
        ),
        _ => None,
    }
}

fn alternatives(bytes: &[u8]) -> Option<Vec<u8>> {
    match bytes {
        packet @ ([0x01, _, _] | [0x02, _, _]) => Some(packet.to_vec()),
        _ => None,
    }
}

fn fixed(bytes: [u8; 2]) -> Vec<u8> {
    let [first, second] = bytes;
    Vec::from([first, second])
}

fn all_lengths(bytes: &[u8]) -> Vec<u8> {
    match bytes {
        [] => Vec::new(),
        [first, ..] => Vec::from([first.to_owned()]),
    }
}

fn covering_rest(bytes: &[u8]) -> Vec<u8> {
    match bytes {
        [remaining @ ..] => remaining.to_vec(),
    }
}

fn slice_guard(bytes: &[u8], marker: u8) -> Option<Vec<u8>> {
    match bytes {
        [first, remaining @ ..] if first == &marker => Some(remaining.to_vec()),
        [_, ..] => None,
        [] => None,
    }
}

fn exact_guard(bytes: &[u8], marker: u8) -> Option<Vec<u8>> {
    match bytes {
        [first, second] if first == &marker => Some(Vec::from([second.to_owned()])),
        [_, ..] => None,
        [] => None,
    }
}

fn suffix_guard(bytes: &[u8], marker: u8) -> Option<Vec<u8>> {
    match bytes {
        [0x01, middle @ .., last] if last == &marker => Some(middle.to_vec()),
        [0x01, ..] => None,
        _ => None,
    }
}

fn all_rest_guard(bytes: &[u8], marker: u8) -> Option<Vec<u8>> {
    match bytes {
        [first, ..] if first == &marker => Some(bytes.to_vec()),
        [all @ ..] => None,
    }
}
