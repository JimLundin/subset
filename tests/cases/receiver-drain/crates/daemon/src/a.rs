fn immediate(events: &mut tokio::sync::mpsc::Receiver<Event>) -> Option<Event> {
    std::iter::from_fn(|| events.try_recv().ok()).find_map(|event| Some(event))
}

fn grouped(events: &mut tokio::sync::mpsc::Receiver<Event>) -> Option<Event> {
    ((std::iter::from_fn)((|| (events.try_recv().ok())))).find_map(|event| Some(event))
}

fn moved(events: &mut tokio::sync::mpsc::Receiver<Event>) -> Option<Event> {
    std::iter::from_fn(move || events.try_recv().ok()).find_map(|event| Some(event))
}

fn field(source: &mut External) -> Option<Event> {
    std::iter::from_fn(|| source.events.try_recv().ok()).find_map(|event| Some(event))
}

fn consumed_result(events: &mut tokio::sync::mpsc::Receiver<Event>) -> Option<Event> {
    let result = std::iter::from_fn(|| events.try_recv().ok()).find_map(|event| match event {
        Event::Wanted => Some(event),
        Event::Other => None,
    });
    result
}

fn disambiguated(events: &mut ::tokio::sync::mpsc::Receiver<Event>) -> Option<Event> {
    ::std::iter::from_fn(|| events.try_recv().ok()).find_map(|event| Some(event))
}
