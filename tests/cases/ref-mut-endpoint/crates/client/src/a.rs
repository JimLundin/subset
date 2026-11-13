pub(crate) enum SendHalf {
    Iroh(SendStream),
}

impl SendHalf {
    pub(crate) async fn write(&mut self, frame: &[u8]) -> Result<(), WireError> {
        match self {
            Self::Iroh(send) => send.write_all(frame).await?,
        }
        Ok(())
    }
}

async fn rpc(transport: Transport, frame: Vec<u8>) -> Result<(), WireError> {
    let mut send = transport.open().await?;
    send.write(&frame).await?;
    Ok(())
}
