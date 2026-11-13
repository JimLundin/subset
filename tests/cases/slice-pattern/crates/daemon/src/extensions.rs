fn nested_tuple_rest(value: External) -> External {
    match value {
        [whole @ (_, ..)] => whole,
        _ => external(),
    }
}

fn nested_tuple_struct_rest(value: External) -> External {
    match value {
        [whole @ Some(..)] => whole,
        _ => external(),
    }
}
