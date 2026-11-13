use std::time::Duration;

struct RetrySchedule { delays: [Duration; 4] }

fn schedule() -> RetrySchedule {
    RetrySchedule { delays: [Duration::ZERO, Duration::from_secs(1), Duration::from_secs(2), Duration::from_secs(4)] }
}

enum Provider { Claude, Codex }
const COUNT: usize = 2;
const PROVIDERS: &[Provider] = &[Provider::Claude, Provider::Codex];

fn providers() -> [Provider; COUNT] {
    let providers = [Provider::Claude, Provider::Codex];
    providers
}

fn labels(first: String, second: String) -> [String; 2] { [first, second] }

fn payload(bytes: &[u8]) -> Option<&[u8]> {
    match bytes { [0x01, remaining @ ..] => Some(remaining), _ => None }
}

struct Packet { bytes: Vec<u8> }
enum MissingPayload { Empty }
impl Packet {
    fn payload(&self) -> Option<&[u8]> { payload(self.bytes.as_slice()) }
    fn required(&self) -> Result<&[u8], MissingPayload> {
        payload(self.bytes.as_slice()).ok_or(MissingPayload::Empty)
    }
}

struct Step<Id> { provider: Id, request: String }
enum Decision<Id> { Approved(Step<Id>), Rejected(Id) }
impl<Id> Step<Id> {
    fn new(provider: Id, request: String) -> Self { Self { provider, request } }
    fn with_request(self, request: String) -> Self { Self { request, ..self } }
    fn provider(self) -> Id { self.provider }
}
fn keep<Id>(step: Step<Id>) -> Step<Id> { step }
struct Related<Left, Right> { left: Left, right: Right }
fn related<Left, Right>(left: Left, right: Right) -> Related<Left, Right> { Related { left, right } }
fn view<T>(values: &[T]) -> Option<&[T]> {
    match values { [_, remaining @ ..] => Some(remaining), [] => None }
}
