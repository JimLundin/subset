fn send(client: &Client) {
    client.send_all([(String::from("a"), 1, Kind::A)]);
}

fn pairs() -> Vec<Pair> {
    Vec::from([(String::from("a"), Kind::A), (String::from("b"), Kind::B)])
        .into_iter()
        .map(Pair::from)
        .collect()
}
