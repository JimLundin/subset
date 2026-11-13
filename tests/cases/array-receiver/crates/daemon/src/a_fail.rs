fn mutable_borrow() -> usize {
    (&mut ["a", "b"]).len()
}

fn indexing() -> String {
    [String::new(), String::new()][0].clone()
}

fn tuple_rows() -> External {
    [(1, 2, 3), (4, 5, 6)].into_iter()
}

fn nested_tuple_rows() -> External {
    ([(1, (2, 3)), (4, (5, 6))]).iter()
}

fn grouped_tuple_rows() -> External {
    [((1, 2, 3)), ((4, 5, 6))].iter()
}

fn grouped_nested_rows() -> External {
    [(1, ((2, 3))), (4, ((5, 6)))].iter()
}

fn comparison_outside_predicate() -> External {
    ["a", "b"].map(|value| value == "a")
}

fn non_literal_repeat(fill: u8) -> Vec<u8> {
    [fill; 4].to_vec()
}
