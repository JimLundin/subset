use r#std::r#iter::r#from_fn;

fn raw_imported() -> Option<Event> {
    from_fn(pull).find_map(|event| Some(event))
}
