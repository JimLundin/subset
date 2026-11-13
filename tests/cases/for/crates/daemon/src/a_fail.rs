async fn emit(log: &Log, kinds: Vec<EventKind>) {
    for kind in kinds {
        let _ = log.append(kind).await;
    }
}

impl Session {
    async fn perform(&self, effects: Vec<Effect>) -> Result<(), Stopped> {
        n0_future::stream::iter(effects)
            .then(|effect| self.effect(effect))
            .try_collect()
            .await
    }

    async fn emit_all(&self, kinds: Vec<EventKind>) -> Result<Vec<Seq>, Stopped> {
        n0_future::stream::iter(kinds.clone())
            .then(|kind| self.emit(kind))
            .try_collect::<Vec<()>>()
            .await?;
        n0_future::stream::iter(kinds)
            .then(|kind| self.seq(kind))
            .try_collect()
            .await
    }
}
