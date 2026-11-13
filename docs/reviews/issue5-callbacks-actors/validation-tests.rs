// Appended only to the scratch agents/src/claude/session.rs for validation.
#[cfg(test)]
mod issue5_from_fn_review_tests {
    use super::{ClaudeEvent, SessionId, refusal};
    use tokio::sync::mpsc;

    fn finished(errors: &[&str]) -> ClaudeEvent {
        ClaudeEvent::TurnFinished {
            subtype: String::from("error_during_execution"),
            outcome: proto::TurnOutcome::Failed { message: String::from("review") },
            terminal_reason: None,
            session_id: SessionId(String::from("review")),
            errors: errors.iter().map(|error| error.to_string()).collect(),
        }
    }

    fn stderr() -> tokio::task::JoinHandle<Option<String>> {
        tokio::spawn(async { Some(String::from("fallback")) })
    }

    #[tokio::test]
    async fn empty_open_channel_falls_back_without_waiting_for_a_sender() {
        let (_sender, mut events) = mpsc::channel(4);
        let result = tokio::time::timeout(
            std::time::Duration::from_secs(5),
            refusal(&mut events, stderr()),
        )
        .await
        .unwrap();
        assert_eq!(result, "fallback");
    }

    #[tokio::test]
    async fn disconnected_channel_uses_the_same_fallback() {
        let (_, mut events) = mpsc::channel(4);
        assert_eq!(refusal(&mut events, stderr()).await, "fallback");
    }

    #[tokio::test]
    async fn first_nonempty_refusal_skips_other_events_and_keeps_later_events() {
        let (sender, mut events) = mpsc::channel(4);
        for event in [
            ClaudeEvent::Unknown(serde_json::Value::Null),
            finished(&[]),
            finished(&["first", "second"]),
            finished(&["later"]),
        ] {
            sender.try_send(event).unwrap();
        }
        assert_eq!(refusal(&mut events, stderr()).await, "first; second");
        let remaining = match events.try_recv().unwrap() {
            ClaudeEvent::TurnFinished { errors, .. } => errors,
            _ => Vec::new(),
        };
        assert_eq!(remaining, Vec::from([String::from("later")]));
        assert_eq!(events.try_recv().err(), Some(mpsc::error::TryRecvError::Empty));
    }
}
