struct Row {
    value: String,
}

fn local_calls(prefix: &str) -> (Row, Row) {
    let row = |value| Row { value: format!("{prefix}{value}") };
    (row("a"), row("b"))
}

fn adapters(first: Vec<String>, second: Vec<String>) -> (Vec<Row>, Vec<Row>) {
    let row = |value| Row { value };
    (
        first.into_iter().map(&row).collect(),
        second.into_iter().map(row).collect(),
    )
}

fn nested_scopes(value: Option<String>) -> Row {
    let row = (|value| Row { value });
    match value {
        Some(value) => { (row)(value) },
        None => row(String::new()),
    }
}

fn shadowed_local() -> Row {
    let row = |value| Row { value };
    let result = row(String::new());
    let row = String::new();
    consume(row);
    result
}

fn shadowed_patterns(values: Vec<String>, value: Option<String>) -> Row {
    let row = |value| Row { value };
    let result = row(String::new());
    let _ = values.iter().map(|row| row.clone()).collect::<Vec<String>>();
    let _ = match value {
        Some(row) => row,
        None => String::new(),
    };
    if let Some(row) = Some(String::new()) {
        consume(row);
    }
    for row in values {
        consume(row);
    }
    result
}

fn shadowed_nested_block() -> (Row, Row) {
    let row = |value| Row { value };
    let first = {
        let row = |value| Row { value };
        row(String::new())
    };
    (first, row(String::new()))
}

fn listed_call() -> External {
    let build = |reply| Msg::Value { reply };
    ask(build)
}

fn immutable_capture(value: String) -> Row {
    let row = move || Row { value };
    row()
}

fn typed_parameter(value: &str) -> Row {
    let row = |value: &str| Row { value: String::from(value) };
    row(value)
}

#[test]
fn calls_in_assertions() {
    let row = |value| Row { value };
    assert_eq!(row(String::new()).value, String::new());
}

fn literal_formatting() -> String {
    let row = |value| Row { value };
    let _ = row(String::new());
    let rowboat = String::new();
    format!("{rowboat} {{row}}")
}

fn shadowed_item() -> Row {
    let row = |value| Row { value };
    let built = row(String::new());
    let _ = {
        fn row() -> String { String::new() }
        row
    };
    built
}
