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

fn unrelated_inner_binding(events: &mut tokio::sync::mpsc::Receiver<Event>) -> Option<Event> {
    {
        let events = Receiver::new();
        let _ = events;
    }
    std::iter::from_fn(|| events.try_recv().ok()).find_map(|event| Some(event))
}

fn later_binding(events: &mut tokio::sync::mpsc::Receiver<Event>) -> Option<Event> {
    let result = std::iter::from_fn(|| events.try_recv().ok()).find_map(|event| Some(event));
    let events = Receiver::new();
    let _ = events;
    result
}

fn external_shadow(events: Receiver, source: &mut tokio::sync::mpsc::Receiver<Event>) -> Option<Event> {
    let events = source;
    std::iter::from_fn(|| events.try_recv().ok()).find_map(|event| Some(event))
}

fn tuple_external((events,): (&mut tokio::sync::mpsc::Receiver<Event>,)) -> Option<Event> {
    std::iter::from_fn(|| events.try_recv().ok()).find_map(|event| Some(event))
}

fn raw_external(r#events: &mut tokio::sync::mpsc::Receiver<Event>) -> Option<Event> {
    std::iter::r#from_fn(|| events.try_recv().ok()).find_map(|event| Some(event))
}

fn closure_shadow(events: Receiver, sources: Vec<&mut tokio::sync::mpsc::Receiver<Event>>) -> Option<Event> {
    sources.into_iter().find_map(|events| {
        std::iter::from_fn(|| events.try_recv().ok()).find_map(|event| Some(event))
    })
}

fn arm_shadow(events: Receiver, source: Option<&mut tokio::sync::mpsc::Receiver<Event>>) -> Option<Event> {
    match source {
        Some(events) => std::iter::from_fn(|| events.try_recv().ok()).find_map(|event| Some(event)),
        None => None,
    }
}

fn if_let_shadow(events: Receiver, source: Option<&mut tokio::sync::mpsc::Receiver<Event>>) {
    if let Some(events) = source {
        let _ = std::iter::from_fn(|| events.try_recv().ok()).find_map(|event| Some(event));
    }
}
