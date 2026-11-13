fn watch_live(live: watch::Receiver<bool>) -> Watcher {
    Watcher::new(live)
}
