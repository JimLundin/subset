enum Provider { Claude, Codex }
fn reference(provider: &Provider) -> u8 {
    match provider { &Provider::Claude => 1, &Provider::Codex => 2 }
}
fn tuple(value: (u8, u8, u8)) -> u8 {
    let (first, ..) = value;
    first
}
fn ends(value: (u8, u8, u8), expected: u8) -> Option<u8> {
    match value {
        (first, .., last) if first == expected => Some(last),
        (_, ..) => None,
    }
}
fn shorter_fallback(value: (u8, u8, u8), expected: u8) -> Option<u8> {
    match value {
        (n @ 1..=3, _, _) if n == expected => Some(n),
        (0..=4, ..) => None,
        _ => None,
    }
}
fn option(value: Option<String>) -> Option<String> {
    match value { whole @ Some(..) => whole, None => None }
}
struct Message { code: u8 }
fn borrowed(value: &Option<Message>) -> Option<u8> {
    match value { &Some(Message { code }) => Some(code), &None => None }
}
fn shared_guard(value: &Option<u8>, expected: u8) -> Option<u8> {
    match value {
        &Some(n @ 1..=3) if n == expected => Some(n),
        &Some(0..=4) => None,
        _ => None,
    }
}
