fn close(rtc: &Rtc, ids: Vec<ChannelId>) {
    for id in ids {
        rtc.close_data_channel(id);
    }
}

fn exited(pending: Pending) {
    for (_, reply) in pending.drain() {
        let _ = reply.send(Err(Error::Exited));
    }
}

fn write(channel: &Channel, frame: &[u8]) -> Result<(), String> {
    for chunk in frame.chunks(CHUNK_LEN) {
        channel.send(chunk).map_err(js)?;
    }
    Ok(())
}

async fn live(log: &Log, kinds: Vec<Kind>) -> Result<(), Stopped> {
    for kind in kinds.into_iter().filter(|k| matches!(k, Kind::Live)) {
        log.append(kind).await?;
    }
    Ok(())
}

async fn kept(log: &Log, kinds: Vec<Kind>) -> Result<Vec<Seq>, Stopped> {
    n0_future::stream::iter(kinds)
        .then(|kind| log.append(kind))
        .try_collect::<Vec<Seq>>()
        .await
}
