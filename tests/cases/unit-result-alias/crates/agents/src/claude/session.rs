pub type Result<T, E = Error> = std::result::Result<T, E>;

fn stop() -> Result<()> {
    Ok(())
}
