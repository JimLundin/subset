pub(crate) struct Attempt {
    endpoint: Endpoint,
    connecting: Connecting,
}

impl Attempt {
    fn start(endpoint: Endpoint) -> Attempt {
        Attempt { connecting: endpoint.connect(), endpoint }
    }
}

pub(crate) enum RecvHalf {
    Iroh(RecvStream),
}
