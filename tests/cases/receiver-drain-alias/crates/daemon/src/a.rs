use std::iter::from_fn as construct;
use std::iter as iteration;
use std as standard;

fn function_alias(events: &mut External) -> External {
    construct(pull).find_map(|event| Some(event))
}

fn module_alias(events: &mut External) -> External {
    iteration::from_fn(pull).find_map(|event| Some(event))
}

fn root_alias(events: &mut External) -> External {
    standard::iter::from_fn(pull).find_map(|event| Some(event))
}

fn value_alias() -> External {
    let constructor = iteration::from_fn;
    constructor(pull).find_map(|event| Some(event))
}
