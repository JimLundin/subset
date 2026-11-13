mod std {
    pub(crate) mod iter {
        pub(crate) fn from_fn(source: External) -> External { source }
    }
}

fn shadowed(events: &mut External) -> External {
    std::iter::from_fn(|| events.try_recv().ok()).find_map(|event| Some(event))
}
