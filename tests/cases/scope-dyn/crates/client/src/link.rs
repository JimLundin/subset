fn listen(on_open: Closure<dyn FnMut(Event)>) -> Listener {
    Listener::new(on_open)
}
