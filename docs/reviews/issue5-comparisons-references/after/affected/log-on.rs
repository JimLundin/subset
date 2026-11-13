    fn on(&mut self, msg: LogMsg) {
        match msg {
            // Live only, so it takes no seq (see `EventKind::AssistantDelta`).
            LogMsg::Append {
                session,
                kind: kind @ (EventKind::AssistantDelta { .. } | EventKind::ReasoningDelta { .. }),
            } => {
                let _ = self.live.send(Event {
                    seq: self.head(),
                    session,
                    kind,
                });
            }
            LogMsg::Append { session, kind } => {
                let event = Event {
                    seq: self.head().next(),
                    session,
                    kind,
                };
                self.events.push(event.clone());
                // No receivers is fine: nobody is subscribed yet.
                let _ = self.live.send(event);
            }
            LogMsg::Welcome { providers, reply } => {
                let _ = reply.send(Welcome {
                    host: self.name.clone(),
                    epoch: self.epoch,
                    head_seq: self.head(),
                    providers,
                });
            }
            LogMsg::Subscribe { after, reply } => {
                let _ = reply.send(Subscription {
                    live: self.live.subscribe(),
                    replay: self
                        .events
                        .iter()
                        .filter(|event| event.seq > after)
                        .cloned()
                        .collect(),
                });
            }
        }
    }
