#[derive(Serialize)]
struct Total {
    n: u8,
}

impl Drop for Total {
    fn drop(&mut self) {}
}

fn list() -> Output {
    let mut cmd = Command::new("ls");
    cmd.output()
}

fn watch() -> Pair {
    watch::channel(false)
}

fn drive(rx: Receiver) -> Total {
    let mut state = Total { n: 0 };
    loop {
        state = rx.recv();
    }
}
