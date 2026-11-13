const STUN_SERVERS: &[&str] = &["stun:a:3478", "stun:b:3478"];

fn replay(name: &str) -> Recording {
    recording(name, &[Frame::Hello, Frame::Bye])
}
