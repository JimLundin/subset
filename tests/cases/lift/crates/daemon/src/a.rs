fn submit(key: Key, composing: Composing) -> Option<Key> {
    Some(key).filter(|key| key.is_enter() && !key.shift() && !composing.active())
}

fn probe(probes: &Probes, trans: Trans, source: SocketAddr) -> Option<&Probe> {
    probes.get(&trans).filter(|probe| probe.addr == source)
}

fn failed(result: &ToolResult) -> Option<Outcome> {
    result.is_error.then_some(Outcome::Failed)
}
