#[cfg(unix)]
fn socket() -> Socket {
    Socket::Unix
}

#[cfg(not(unix))]
fn socket() -> Socket {
    Socket::Tcp
}
