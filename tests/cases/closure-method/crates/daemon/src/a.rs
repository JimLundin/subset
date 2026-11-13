fn last_seen(events: &[Event], seq: HostSeq) -> Option<&Event> {
    events.iter().rfind(|e| e.seq == seq)
}

async fn closed(mut state: watch::Receiver<ChannelState>) -> Result<(), Lost> {
    state.wait_for(|s| s == &ChannelState::Closed).await.map_err(Lost::from_watch)?;
    Ok(())
}

fn label(name: Option<Name>) -> Option<Label> {
    name.map(|n| n.label())
}
