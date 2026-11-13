fn stored(events: &mut External) -> External {
    let generator = std::iter::from_fn(|| events.try_recv().ok());
    generator.find_map(|event| Some(event))
}

fn returned(events: &mut External) -> External {
    std::iter::from_fn(|| events.try_recv().ok())
}

fn passed(events: &mut External) -> External {
    consume(std::iter::from_fn(|| events.try_recv().ok()))
}

fn lazy(events: &mut External) -> External {
    std::iter::from_fn(|| events.try_recv().ok()).map(|event| event)
}

fn intervening(events: &mut External) -> External {
    std::iter::from_fn(|| events.try_recv().ok()).take(1).find_map(|event| Some(event))
}

fn other_terminal(events: &mut External) -> External {
    std::iter::from_fn(|| events.try_recv().ok()).collect::<Vec<Event>>()
}

fn blocking(events: &mut External) -> External {
    std::iter::from_fn(|| events.blocking_recv()).find_map(|event| Some(event))
}

fn block_body(events: &mut External) -> External {
    std::iter::from_fn(|| { events.try_recv().ok() }).find_map(|event| Some(event))
}

fn argument(events: &mut External) -> External {
    std::iter::from_fn(|value| events.try_recv().ok()).find_map(|event| Some(event))
}

fn named(events: &mut External) -> External {
    let pull = || events.try_recv().ok();
    std::iter::from_fn(pull).find_map(|event| Some(event))
}

fn named_function() -> External {
    std::iter::from_fn(pull).find_map(|event| Some(event))
}

fn referenced_callback(events: &mut External) -> External {
    std::iter::from_fn(&|| events.try_recv().ok()).find_map(|event| Some(event))
}

fn async_pull(events: &mut External) -> External {
    std::iter::from_fn(async || events.try_recv().ok()).find_map(|event| Some(event))
}

fn extra_pull_argument(events: &mut External) -> External {
    std::iter::from_fn(|| events.try_recv(1).ok()).find_map(|event| Some(event))
}

fn receiver_factory() -> External {
    std::iter::from_fn(|| factory().try_recv().ok()).find_map(|event| Some(event))
}

fn extra_mapping(events: &mut External) -> External {
    std::iter::from_fn(|| events.try_recv().ok().map(|event| event)).find_map(|event| Some(event))
}

fn qualified_constructor(events: &mut External) -> External {
    std::iter::from_fn::<Event, _>(|| events.try_recv().ok()).find_map(|event| Some(event))
}

fn named_consumer(events: &mut External) -> External {
    std::iter::from_fn(|| events.try_recv().ok()).find_map(chosen)
}

fn qualified_consumer(events: &mut External) -> External {
    std::iter::from_fn(|| events.try_recv().ok()).find_map::<Event, _>(|event| Some(event))
}

fn no_conversion(events: &mut External) -> External {
    std::iter::from_fn(|| events.try_recv()).find_map(|event| Some(event))
}

fn unsupported_callee(events: &mut External) -> External {
    external::from_fn(|| events.try_recv().ok()).find_map(|event| Some(event))
}

fn aliased_function(events: &mut External) -> External {
    let construct = std::iter::from_fn;
    construct(|| events.try_recv().ok()).find_map(|event| Some(event))
}

fn passed_function() -> External {
    register(std::iter::from_fn)
}

fn own_receiver(events: Receiver) -> External {
    std::iter::from_fn(|| events.try_recv().ok()).find_map(|event| Some(event))
}

fn own_constructed_receiver() -> External {
    let events = Receiver::new();
    std::iter::from_fn(|| events.try_recv().ok()).find_map(|event| Some(event))
}

fn grouped_own_receiver(events: Receiver) -> External {
    std::iter::from_fn(|| (events).try_recv().ok()).find_map(|event| Some(event))
}

fn grouped_constructor_escape(events: &mut External) -> External {
    (std::iter::from_fn)(pull)
}

fn qualified_pull(events: &mut External) -> External {
    std::iter::from_fn(|| events.try_recv::<Event>().ok()).find_map(|event| Some(event))
}

fn qualified_conversion(events: &mut External) -> External {
    std::iter::from_fn(|| events.try_recv().ok::<Event>()).find_map(|event| Some(event))
}

struct Receiver {}
