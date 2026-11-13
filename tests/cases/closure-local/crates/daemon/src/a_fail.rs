fn returned() -> External {
    let row = |value| Row { value };
    row
}

fn aliased() -> Row {
    let row = |value| Row { value };
    let alias = row;
    alias(String::new())
}

fn stored() -> External {
    let row = |value| Row { value };
    Holder { build: row }
}

fn unlisted_call() -> External {
    let row = |value| Row { value };
    register(row)
}

fn unlisted_method(receiver: External) -> External {
    let row = |value| Row { value };
    receiver.register(row)
}

fn captured() -> External {
    let row = |value| Row { value };
    ask(|| row(String::new()))
}

fn async_capture() -> External {
    let row = |value| Row { value };
    tokio::spawn(async move { row(String::new()) })
}

fn macro_capture() -> String {
    let row = |value| Row { value };
    let _ = row(String::new());
    format!("{row:?}")
}

fn macro_argument() -> String {
    let row = |value| Row { value };
    format!("{:?}", row)
}

fn unused() -> String {
    let row = |value| Row { value };
    String::new()
}

fn mutable_binding() -> Row {
    let mut row = |value| Row { value };
    row(String::new())
}

fn named_predicate(values: Vec<u32>) -> Vec<u32> {
    let keep = |value| value == 3;
    values.into_iter().filter(keep).collect()
}

fn named_async(values: External) -> External {
    let next = |value| async move { value };
    values.then(next)
}

fn mixed_uses() -> Row {
    let row = |value| Row { value };
    let built = row(String::new());
    register(&row);
    built
}

fn shadow_initializer() -> Row {
    let row = |value| Row { value };
    let row = save(row);
    row(String::new())
}

fn escape_inner_block() -> External {
    {
        let row = |value| Row { value };
        row
    }
}

fn tuple_binding() -> Row {
    let (row, value) = (|value| Row { value }, String::new());
    row(value)
}

fn typed_let() -> Row {
    let row: Row = build();
    row
}

fn inline_typed(values: Vec<String>) -> Vec<Row> {
    values.into_iter().map(|value: String| Row { value }).collect()
}

fn typed_bool() -> External {
    let keep = |value: bool| value;
    keep(external())
}

fn unknown_macro() -> External {
    let row = |value| Row { value };
    custom!(row(String::new()))
}
