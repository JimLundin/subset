mod std {
    pub(crate) mod iter {
        pub(crate) fn from_fn(source: External) -> External { source }
    }
}

fn shadowed(events: &mut External) -> External {
    std::iter::from_fn(|| events.try_recv().ok()).find_map(|event| Some(event))
}

fn core_call(source: External) -> External {
    std::iter::from_fn(source)
}

fn core_function_value(source: External) -> External {
    let construct = std::iter::from_fn;
    construct(source)
}

fn absolute_external(events: &mut tokio::sync::mpsc::Receiver<Event>) -> Option<Event> {
    ::std::iter::from_fn(|| events.try_recv().ok()).find_map(|event| Some(event))
}

mod nested {
    mod r#std {
        pub(crate) mod iter {
            pub(crate) fn from_fn(value: External) -> External { value }
        }
    }

    fn raw_shadowed(events: &mut External) -> External {
        std::iter::from_fn(|| events.try_recv().ok()).find_map(|event| Some(event))
    }

    fn raw_core(source: External) -> External {
        r#std::iter::r#from_fn(source)
    }
}
