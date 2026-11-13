use n0_future::stream::iter;

impl SessionActor {
    async fn settled(&self, kinds: Vec<EventKind>) -> Result<(), Stopped> {
        for kind in kinds {
            self.emit({
                self.turn = Turn::Idle;
                kind
            })
            .await?;
        }
        Ok(())
    }

    async fn idle(&self, calls: Vec<Call>) -> Result<(), Stopped> {
        for call in {
            self.turn = Turn::Idle;
            calls
        } {
            self.emit(call).await?;
        }
        Ok(())
    }

    async fn chained(&self, keys: Vec<Key>) -> Result<(), Stopped> {
        for key in keys {
            self.lookup(key.clone()).await?.store(key).await?;
        }
        Ok(())
    }

    async fn each(&self, kinds: Vec<EventKind>) -> Result<(), Stopped> {
        n0_future::stream::iter(kinds)
            .map(Ok)
            .try_for_each(|kind| self.emit(kind))
            .await
    }

    async fn unit(&self, kinds: Vec<EventKind>) -> Result<(), Stopped> {
        let () = n0_future::stream::iter(kinds)
            .then(|kind| self.emit(kind))
            .try_collect()
            .await?;
        Ok(())
    }

    async fn imported(&self, kinds: Vec<EventKind>) -> Result<(), Stopped> {
        iter(kinds).then(|kind| self.emit(kind)).try_collect().await
    }

    async fn through(&self, kinds: Vec<EventKind>) -> Result<(), Stopped> {
        let steps = n0_future::stream::iter(kinds).then(|kind| self.emit(kind));
        steps.try_collect().await
    }

    async fn mapped(&self, kinds: Vec<EventKind>) -> Result<(), Lost> {
        n0_future::stream::iter(kinds)
            .then(|kind| self.emit(kind))
            .try_collect::<Vec<Done>>()
            .await
            .map(|_| ())
            .map_err(Lost::from)
    }

    fn close(&self, ids: Vec<ChannelId>) {
        ids.into_iter().for_each(|id| self.rtc.close_data_channel(id));
    }
}
