fn send(frame: Frame) -> Result<(), Lost> {
    #[cfg(unix)]
    let socket = frame.socket();
    Ok(())
}
