#[cfg(unix)]
fn socket() -> Socket {
    Socket::Unix
}

#[cfg(not(unix))]
fn listen() -> Socket {
    Socket::Tcp
}
