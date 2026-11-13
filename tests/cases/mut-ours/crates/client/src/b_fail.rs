use crate::a::Attempt;

async fn race(mut first: Attempt) -> Result<Connection, Lost> {
    first.connecting.poll().await
}

async fn dial(endpoint: Endpoint) -> Result<Connection, Lost> {
    let mut first = Attempt::start(endpoint);
    first.connecting.poll().await
}

fn built(endpoint: Endpoint, connecting: Connecting) -> Attempt {
    let mut attempt = Attempt { endpoint, connecting };
    attempt.connecting.reset();
    attempt
}
