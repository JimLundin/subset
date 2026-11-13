fn stored(first: String, second: String) -> Vec<String> {
    let values = [first, second];
    values.to_vec()
}

fn returned() -> [String; 2] {
    [String::from("a"), String::from("b")]
}

fn field() -> Holder {
    Holder { values: [String::from("a"), String::from("b")] }
}

struct Holder { values: [String; 2] }
