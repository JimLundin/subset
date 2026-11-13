fn rename(host: Host, name: String) -> Host {
    Host { name, ..host }
}

fn options(model: String) -> Options {
    Options { model, ..Options::new() }
}

fn live(outbox: &Outbox) -> Status {
    Status { state: LinkState::Live, ..outbox.current() }
}
