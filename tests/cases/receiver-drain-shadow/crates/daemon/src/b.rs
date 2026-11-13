mod std {
    pub(crate) mod iter {
        pub(crate) fn from_fn(source: External) -> External { source }
    }
}

fn absolute_boundary(events: &mut tokio::sync::mpsc::Receiver<Event>) -> Option<Event> {
    ::std::iter::from_fn(|| events.try_recv().ok()).find_map(|event| Some(event))
}

fn own_import(source: External) -> External {
    use std::iter::from_fn;
    from_fn(source)
}
