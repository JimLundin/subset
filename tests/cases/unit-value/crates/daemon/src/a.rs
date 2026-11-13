fn ready(tx: oneshot::Sender<()>) -> Result<(), Lost> {
    tx.send(()).map_err(Lost::from_unit)?;
    Ok(())
}
