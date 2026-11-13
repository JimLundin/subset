struct Session {
    live: bool,
}

fn is_live(session: &Session) -> bool {
    session.live
}
