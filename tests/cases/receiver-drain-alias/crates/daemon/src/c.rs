use std::iter::{self};

fn grouped_import() -> Option<Event> {
    iter::from_fn(pull).find_map(|event| Some(event))
}

fn grouped_value() -> External {
    register(iter::from_fn)
}

fn raw_callback() -> Option<Event> {
    std::iter::r#from_fn(pull).find_map(|event| Some(event))
}

fn raw_value() -> External {
    register(r#std::r#iter::r#from_fn)
}
