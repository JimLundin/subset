fn open(path: &Path) -> Result<File, Lost> {
    OpenOptions::new().create(true).open(path).map_err(Lost::Open)
}

fn init() -> Capabilities {
    Capabilities { experimental_api: Some(true) }
}

fn idle() -> Watch {
    Watch::new(watch::channel(false))
}
