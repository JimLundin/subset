fn kind(message: &[u8]) -> Option<u16> {
    let (kind, _) = message.split_at_checked(2)?;
    Some(u16::from_be_bytes(kind.try_into().ok()?))
}
