struct ClaudeId(String);
struct CodexId(String);
struct Step<Id> { provider: Id }
fn claude(step: Step<ClaudeId>) -> ClaudeId { step.provider }
fn wrong(step: Step<CodexId>) -> ClaudeId { claude(step) }
