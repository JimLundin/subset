async fn lost(mut peer: watch::Receiver<Peer>) -> Result<(), Lost> {
    let _ = peer.wait_for(|p| Peer::Lost.eq(p)).await.map_err(Lost::from_watch)?;
    Ok(())
}

fn probe(probes: &Probes, trans: Trans, source: SocketAddr) -> Option<&Probe> {
    let probe = probes.get(&trans).filter(|probe| probe.addr == source)?;
    Some(probe)
}

fn both(x: Event, b: Seq, d: Name) -> Option<Event> {
    Some(x).filter(|x| x.seq == b && x.name == d)
}

fn origin(ours: Option<Ours>, event: &Event) -> Option<Ordering> {
    ours.filter(|ours| ours.session == event.session)
        .map(|ours| event.seq.cmp(&ours.after))
}
