    async fn on(&mut self, msg: RegistryMsg) -> Result<(), Stopped> {
        match msg {
            RegistryMsg::Start {
                provider,
                directory,
                model,
                effort,
                by,
                reply,
            } => {
                let result = self.start(provider, directory, (model, effort), by).await?;
                let _ = reply.send(result);
            }
            RegistryMsg::Continue { session, reply } => {
                let relaunch = match self.sessions.get(&session) {
                    None => Err(Rejection::UnknownSession),
                    Some((provider, handle)) => {
                        let &provider = provider;
                        self.ready(provider)
                            .map(|(launch, _)| (handle.clone(), launch, self.models(provider)))
                            .map_err(|reason| Rejection::ProviderUnavailable { reason })
                    }
                };
                let _ = reply.send(relaunch);
            }
            RegistryMsg::Find { session, reply } => {
                let found = self
                    .sessions
                    .get(&session)
                    .map(|(_, handle)| handle.clone());
                let _ = reply.send(found);
            }
            RegistryMsg::Providers { reply } => {
                let _ = reply.send(self.providers());
            }
            RegistryMsg::SetCredential { credential, reply } => {
                let provider = credential.provider();
                let result = match (credential.check(), &self.stored) {
                    (Err(reason), _) => {
                        CommandResult::Rejected(Rejection::InvalidCredential { reason })
                    }
                    // Saving now would overwrite the other provider's.
                    (Ok(()), Stored::Unreadable(reason)) => {
                        CommandResult::Rejected(Rejection::CredentialNotSaved {
                            reason: reason.clone(),
                        })
                    }
                    (Ok(()), Stored::Opened(credentials)) => {
                        let next = credentials.clone().set(credential);
                        self.store(provider, next).await?
                    }
                };
                let _ = reply.send(result);
            }
            RegistryMsg::ClearCredential { provider, reply } => {
                let result = match self.readable() {
                    Ok(credentials) => self.store(provider, credentials.clear(provider)).await?,
                    Err(reason) => {
                        CommandResult::Rejected(Rejection::CredentialNotSaved { reason })
                    }
                };
                let _ = reply.send(result);
            }
        }
        Ok(())
    }
