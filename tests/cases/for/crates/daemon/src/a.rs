fn record(sizes: Vec<Size>, meter: &Meter) {
    for size in sizes {
        meter.record(size);
    }
}

impl Session {
    async fn apply(&self, steps: Vec<Step>) -> Result<(), Stopped> {
        for step in steps {
            self.step(step).await?;
        }
        Ok(())
    }
}

async fn append(log: &Log, kinds: Vec<EventKind>) -> Result<(), Stopped> {
    for kind in kinds {
        log.append(kind.session(), kind).await?;
        notify(&kind).await?;
    }
    Ok(())
}
