async fn arm(link: Link, state: LinkState) {
    link.update(|s| s.state = state).await;
}

async fn connecting(mut state: watch::Receiver<ChannelState>) {
    let _ = state.wait_until(|s| s != &ChannelState::Connecting).await;
}
