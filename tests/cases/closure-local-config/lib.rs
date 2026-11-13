fn configured(receiver: External) -> External {
    let build = |value| value;
    receiver.configured(build)
}

fn unlisted(receiver: External) -> External {
    let build = |value| value;
    receiver.map(build)
}
