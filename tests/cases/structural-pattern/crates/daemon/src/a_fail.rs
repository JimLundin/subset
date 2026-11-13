fn mutable_reference(value: &mut External) -> External {
    match value { &mut whole => whole }
}
fn borrowed_binding(value: External) -> External {
    match value { ref whole => whole }
}
fn bound_tuple_rest(value: External) -> External {
    match value { (first, tail @ ..) => first }
}
fn narrowed(value: (u8, u8, u8), expected: u8) -> Option<u8> {
    match value {
        (first, ..) if first == expected => Some(first),
        (0, ..) => None,
    }
}
fn dereference(value: &u8) -> u8 { *value }
