fn reference(value: External) -> External {
    match value {
        whole @ &Some(_) => whole,
        _ => external(),
    }
}

fn tuple_rest(value: External) -> External {
    match value {
        whole @ (_, ..) => whole,
    }
}

fn tuple_struct_rest(value: External) -> External {
    match value {
        whole @ Some(..) => whole,
        _ => external(),
    }
}
