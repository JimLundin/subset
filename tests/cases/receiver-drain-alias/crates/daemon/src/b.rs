use std::iter::from_fn;
use std::iter;

fn imported(events: &mut External) -> External {
    from_fn(pull).find_map(|event| Some(event))
}

fn imported_module(events: &mut External) -> External {
    iter::from_fn(pull).find_map(|event| Some(event))
}
