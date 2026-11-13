fn moved(value: &Option<String>) -> String {
    match value { &Some(text) => text, &None => String::new() }
}
