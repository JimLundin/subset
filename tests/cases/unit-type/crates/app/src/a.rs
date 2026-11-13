fn ready(done: oneshot::Sender<()>) -> Result<(), Lost> {
    done.send(()).map_err(Lost::from_unit)
}

fn close_button(on_close: EventHandler<()>) -> Element {
    Element::button(on_close)
}
