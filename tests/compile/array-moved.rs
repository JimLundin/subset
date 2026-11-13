fn moved(values: [String; 2]) -> [String; 2] {
    let consumed = values.into_iter().collect::<Vec<String>>();
    values
}
