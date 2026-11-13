struct Listeners {
    on_open: Box<dyn Fn(Event)>,
}
