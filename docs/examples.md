# Subset patterns by example

What the constructs in `subset.toml` (version 7) look like in practice.
Every example passes `subset check`, and a test in `tools/subset` keeps it
that way. A first-line comment naming a path, such as
`// crates/proto/src/frame.rs`, places an example where a location rule
allows it. A **Note** marks where the subset forces a longer form.

## 1. Two states are an enum, not a `bool`

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Visibility {
    Shown,
    Hidden,
}

pub fn toggle(visibility: Visibility) -> Visibility {
    match visibility {
        Visibility::Shown => Visibility::Hidden,
        Visibility::Hidden => Visibility::Shown,
    }
}
```

## 2. IDs and counters are newtypes, and arithmetic is a checked method

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct HostSeq(u64);

impl HostSeq {
    pub const ZERO: HostSeq = HostSeq(0);

    pub fn next(self) -> Option<HostSeq> {
        self.0.checked_add(1).map(HostSeq)
    }
}
```

There are no `+ - * /` operators, and no `a.add(b)` or `Add::add(a, b)`
either (proposal 0038). Overflow is a `None` the caller has to handle.
`HostSeq` derives `PartialOrd` and `Ord` because its order means
something (proposal 0024). An ID such as `RequestId`, whose order means
nothing, doesn't.

## 3. Comparison is an `Ordering`, matched on the whole newtype

```rust
use std::cmp::Ordering;

pub enum Freshness {
    Behind,
    Current,
    Ahead,
}

impl HostSeq {
    pub fn freshness(self, head: HostSeq) -> Freshness {
        match self.cmp(&head) {
            Ordering::Less => Freshness::Behind,
            Ordering::Equal => Freshness::Current,
            Ordering::Greater => Freshness::Ahead,
        }
    }
}
```

This replaces `if seen < head { … } else if …`. Compare the ordered newtype
itself with `a.cmp(&b)`, never its field with `a.0.cmp(&b.0)` (proposal 0024).

## 4. Choosing by value is a literal pattern

```rust
pub enum Provider {
    ClaudeCode,
    Codex,
}

pub fn provider(name: &str) -> Option<Provider> {
    match name {
        "claude" => Some(Provider::ClaudeCode),
        "codex" => Some(Provider::Codex),
        _ => None,
    }
}
```

This replaces `if name == "claude"`.

## 5. A state machine is an enum with a transition function

```rust
pub enum Link {
    Disconnected { attempts: u32 },
    Connecting { attempts: u32 },
    Live { session: SessionId },
}

pub enum LinkEvent {
    Dialled,
    Opened(SessionId),
    Dropped,
}

impl Link {
    pub fn on(self, event: LinkEvent) -> Link {
        match (self, event) {
            (_, LinkEvent::Opened(session)) => Link::Live { session },
            (Link::Disconnected { attempts } | Link::Connecting { attempts }, LinkEvent::Dialled) => {
                Link::Connecting { attempts }
            }
            (Link::Disconnected { attempts }, LinkEvent::Dropped) => Link::Disconnected { attempts },
            (Link::Connecting { attempts }, LinkEvent::Dropped) => Link::Disconnected {
                attempts: attempts.saturating_add(1),
            },
            (Link::Live { session }, LinkEvent::Dialled) => Link::Live { session },
            (Link::Live { .. }, LinkEvent::Dropped) => Link::Disconnected { attempts: 0 },
        }
    }
}
```

The `match` on `(state, event)` reads as a transition table. Tuples were
added in version 1 (proposal 0001).

## 6. Missing values are `Option`, matched

```rust
pub fn label(host_name: Option<&str>) -> String {
    match host_name {
        Some(name) => format!("host {name}"),
        None => String::from("unnamed host"),
    }
}
```

## 7. Errors are enums, passed on with `?`

```rust
#[derive(Debug)]
pub enum LoadError {
    Read(std::io::Error),
    Parse(toml::de::Error),
}

pub fn load(path: &Path) -> Result<Config, LoadError> {
    let text = std::fs::read_to_string(path).map_err(LoadError::Read)?;
    toml::from_str(&text).map_err(LoadError::Parse)
}
```

A variant constructor such as `LoadError::Read` is passed as a function.
`map_err` is the form to use when one error type can mean different
things in the same enum. When it means one thing, use `#[from]` (§16).

## 8. Collections are transformed with adapters

```rust
pub fn pending(requests: &[Request]) -> Vec<RequestId> {
    requests
        .iter()
        .filter_map(|request| match request.state {
            RequestState::Open => Some(request.id),
            RequestState::Resolved { .. } => None,
        })
        .collect()
}
```

To select and transform in one step, use `filter_map` with a `match`. To
select items without transforming them, or to count them, use `filter` with
a predicate (§22). A function that only reads a sequence takes `&[T]`
(proposal 0014).

## 9. Totals are folds with checked arithmetic

```rust
pub fn total_bytes(frames: &[Frame]) -> Option<u64> {
    frames
        .iter()
        .try_fold(0_u64, |total, frame| total.checked_add(frame.len))
}
```

## 10. Ask for the value, not whether it exists

```rust
pub fn describe(events: &[Event]) -> String {
    match events.last() {
        Some(event) => format!("last event #{}", event.seq.0),
        None => String::from("no events yet"),
    }
}
```

This replaces `if events.is_empty() { … } else { events[events.len() - 1] }`.

## 11. When std only gives a `bool`

```rust
pub enum Workspace {
    Present(PathBuf),
    Missing,
}

pub fn workspace(path: PathBuf) -> Workspace {
    match std::fs::metadata(&path) {
        Ok(_) => Workspace::Present(path),
        Err(_) => Workspace::Missing,
    }
}
```

Prefer the `Result`/`Option` form of an API (`metadata` over `exists()`).
Where only a `bool` exists, `.then_some(value)` turns it into an `Option`
to match on. The reviewer treats repeated use as friction.

## 12. A changed value is a new value

```rust
pub fn rename(host: Host, name: String) -> Host {
    Host { name, ..host }
}
```

Use struct update instead of `let mut host = host; host.name = name;`.

## 13. Logging

```rust
pub fn caught_up(seen: HostSeq, name: &str) {
    tracing::info!(seq = seen.0, host = %name, "caught up");
}
```

## 14. Tests compare values

```rust
#[cfg(test)]
mod tests {
    use super::{toggle, Visibility};

    #[test]
    fn toggling_twice_returns_to_the_start() {
        assert_eq!(toggle(toggle(Visibility::Shown)), Visibility::Shown);
    }
}
```

## 15. An actor owns its state and runs one loop

```rust
use std::convert::Infallible;

use tokio::sync::{mpsc, oneshot};

pub enum Msg {
    Append(Event),
    Head { reply: oneshot::Sender<HostSeq> },
}

/// The inbox closed: nobody can reach the actor any more.
#[derive(Debug)]
pub struct Stopped {}

pub struct Log {
    events: Vec<Event>,
    head: HostSeq,
}

impl Log {
    pub async fn run(mut self, mut inbox: mpsc::Receiver<Msg>) -> Result<Infallible, Stopped> {
        loop {
            let msg = inbox.recv().await.ok_or(Stopped {})?;
            self.on(msg);
        }
    }

    fn on(&mut self, msg: Msg) {
        match msg {
            Msg::Append(event) => {
                self.head = event.seq;
                self.events.push(event);
            }
            Msg::Head { reply } => {
                let _ = reply.send(self.head);
            }
        }
    }
}
```

`run` is `async` because it waits on a channel (proposal 0006). The `loop` is
its whole body: it takes one message and hands it to one method (0007). Only
the actor's own state changes, through `mut self` and `&mut self` (0008).
Every other function still builds new values. The reply's send fails only
when the asker has gone, and there is nothing to do about it, so the result
is discarded with `let _ =` (proposal 0025).

## 16. Errors convert with `?` through `#[from]`

```rust
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("reading the config: {0}")]
    Read(#[from] std::io::Error),
    #[error("parsing the config: {0}")]
    Parse(#[from] toml::de::Error),
}

pub fn load_config(path: &Path) -> Result<Config, ConfigError> {
    let text = std::fs::read_to_string(path)?;
    Ok(toml::from_str(&text)?)
}
```

Each variant has one message (proposal 0010). `#[from]` fits because each
source error type appears in one variant only (0011). Where the same type
means two things, use `map_err` as in §7.

## 17. Wire types derive serde

```rust
// crates/proto/src/frame.rs
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct HostSeq(pub u64);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Frame {
    Hello { host: String },
    Event { seq: HostSeq, body: String },
}
```

Only types that cross a wire derive serde, and they live in `crates/proto` or
an agents protocol module (proposal 0012).

## 18. Platform code comes in pairs

```rust
pub enum Platform {
    Browser,
    Native,
}

#[cfg(target_family = "wasm")]
pub fn platform() -> Platform {
    Platform::Browser
}

#[cfg(not(target_family = "wasm"))]
pub fn platform() -> Platform {
    Platform::Native
}
```

A `cfg` sits on a whole item, and the other platform's item sits next to it
in the same file (proposal 0016).

## 19. Discarding a value is `let _ =`

```rust
use tokio::sync::oneshot;

pub fn answer(reply: oneshot::Sender<HostSeq>, head: HostSeq) {
    let _ = reply.send(head);
}
```

`let _ = value;` is the one way to say "this result doesn't matter". Don't
write `drop(reply.send(head))` or `reply.send(head).ok();`. `drop` is only
for releasing a named value that owns a resource earlier than the end of
its scope (proposal 0025, revised).

## 20. An actor handle asks through one `ask` helper

```rust
use tokio::sync::{mpsc, oneshot};

pub enum LogMsg {
    Head { reply: oneshot::Sender<HostSeq> },
    Len { reply: oneshot::Sender<usize> },
}

/// The actor stopped before it answered.
#[derive(Debug)]
pub struct LogStopped {}

pub struct LogHandle {
    commands: mpsc::Sender<LogMsg>,
}

impl LogHandle {
    pub async fn head(&self) -> Result<HostSeq, LogStopped> {
        self.ask(|reply| LogMsg::Head { reply }).await
    }

    pub async fn len(&self) -> Result<usize, LogStopped> {
        self.ask(|reply| LogMsg::Len { reply }).await
    }

    async fn ask<T>(&self, build: impl FnOnce(oneshot::Sender<T>) -> LogMsg) -> Result<T, LogStopped> {
        let (reply, answer) = oneshot::channel();
        self.commands.send(build(reply)).await.map_err(|_| LogStopped {})?;
        answer.await.map_err(|_| LogStopped {})
    }
}
```

Every request and reply round trip goes through `ask` (proposal 0026).
The closure only builds the message variant. A crate with one handle or
handles that fail differently keeps this private `ask<T>` shape.

A crate with multiple handles that all fail with one stopped-error type
shares one free helper instead (issue #3's amendment). It is private or
`pub(crate)`, with a second unbounded parameter for the message type:

```rust
pub(crate) async fn ask<M, T>(
    inbox: &mpsc::Sender<M>,
    build: impl FnOnce(oneshot::Sender<T>) -> M,
) -> Result<T, Stopped> {
    let (reply, answer) = oneshot::channel();
    inbox.send(build(reply)).await.map_err(|_| Stopped {})?;
    answer.await.map_err(|_| Stopped {})
}

impl Log {
    pub async fn head(&self) -> Result<HostSeq, Stopped> {
        ask(&self.inbox, |reply| LogMsg::Head { reply }).await
    }
}

impl Registry {
    pub async fn find(&self, session: SessionId) -> Result<Option<Session>, Stopped> {
        ask(&self.inbox, |reply| RegistryMsg::Find { session, reply }).await
    }
}
```

The shared helper replaces the per-handle methods. `M` appears only as the
sender's item and the builder's return type. `ask` is the one place outside
a codec where a fn takes type parameters, and the one `impl Trait` argument
in the subset.

## 21. Acting on one case is an `if let` with no `else`

```rust
pub fn announce(host_name: Option<&str>) {
    if let Some(name) = host_name {
        tracing::info!(host = %name, "host named");
    }
}

pub struct Pending {
    reply: oneshot::Sender<HostSeq>,
}

pub fn cancel(open: Option<Pending>) {
    if let Some(Pending { .. }) = open {
        tracing::info!("request cancelled");
    }
}
```

When only one case needs doing and the rest need nothing, write `if let`
with no `else`, no `else if` and no `&&` let-chain (proposal 0027). It
replaces a `match` whose other arm is `None => {}`, which clippy's
`single_match` rejects. As soon as another case needs something, it is a
`match` with every arm written out (§6).

To act only when a value is present, name the payload's pattern with its
fields elided, as `cancel` does. clippy's `redundant_pattern_matching`
rejects `Some(_)`, and `Some(..)` is `pat.rest`, which isn't allowed.

## 22. Counting and searching take a predicate closure

```rust
pub fn open_count(requests: &[Request]) -> usize {
    requests
        .iter()
        .filter(|request| matches!(request.state, RequestState::Open))
        .count()
}

pub fn first_after(events: &[Event], seen: HostSeq) -> Option<&Event> {
    events.iter().find(|event| event.seq > seen)
}
```

A closure passed straight to a method in `predicate_adapters` (`filter`,
`find`, `rfind`, `any`, `all`, `position`, `take_while`, `skip_while`,
`retain`, and tokio's `wait_for`) may have a boolean expression as its body:
`matches!`, a comparison, `&&`, `||` or `!` (proposals 0028 and 0037).
Nowhere else, so `state.wait_for(|s| s == &State::Closed)` reads like
`filter`, not `State::Closed.eq(s)`. `filter(..).count()` is the one way to count, in place of
`filter_map(..).count()` or a `fold` that adds one. A `bool` still never
becomes a binding, a field, a parameter or a return type of our own.

## 23. A stream endpoint changes in place

```rust
pub enum SendHalf {
    Iroh(iroh::endpoint::SendStream),
}

impl SendHalf {
    pub async fn write(&mut self, frame: &[u8]) -> Result<(), WireError> {
        match self {
            Self::Iroh(send) => send.write_all(frame).await?,
        }
        Ok(())
    }
}

pub async fn greet(transport: Transport, hello: &[u8], ready: &[u8]) -> Result<(), WireError> {
    let mut send = transport.open_send().await?;
    send.write(hello).await?;
    send.write(ready).await?;
    send.finish()
}
```

A type on the `endpoints` list in `subset.toml` wraps a library stream
whose own API takes `&mut`, so its methods take `&mut self` and any code
may call them on a `let mut` binding (proposal 0035). Without the list, each
write would take the half by value and hand it back, and every caller would
rebind it. Every other type of ours keeps 0008's reach: its value-returning
methods stay, and no fn takes a `&mut` of it.

## 24. A changed copy of what a method returns

```rust
pub fn catching_up(outbox: &Outbox, host: HostName) -> Status {
    Status {
        host: Some(host),
        state: LinkState::CatchingUp,
        ..outbox.current()
    }
}
```

The base of a struct update may be a method call that returns a fresh
value of the struct's type (proposal 0036). Write the call in place, as
here, rather than binding it with `let current = outbox.current();` first.

## 25. Comparing two runtime values is a guard

```rust
pub fn authenticate(open: Option<Open>, token: AuthToken) -> Result<Hello, Refusal> {
    match open {
        Some(Open::Hello(hello)) if hello.token == token => Ok(hello),
        Some(Open::Hello(_)) => Err(Refusal::WrongToken),
        Some(Open::Subscribe(_)) | None => Err(Refusal::NoHello),
    }
}

pub fn submits(key: Key, composer: &Composer) -> Option<Key> {
    Some(key).filter(|key| key.is_enter() && !key.shift_key() && !composer.is_composing())
}
```

A pattern can't name a runtime value, so an arm that depends on a bound
value being equal to, or ordered against, another runtime value takes a
guard (proposal 0040). The guard is one comparison or one `matches!`, with
no `&&`, `||` or `!`, and a later arm with no guard covers the same
pattern, so the otherwise case is written out: Rust's exhaustiveness check
ignores guards. `Some(hello).filter(|hello| hello.token == token)` and
`hello.token.eq(&token).then_some(hello)` are violations. A lift that
combines library booleans, as `submits` does, stays, since a guard can't
hold `&&`.

The guard tests a value the arm's pattern bound. A guard that compares
with a literal, a variant or a constant is a pattern: `Some(Seq(0))`, not
`Some(seq) if seq == Seq(0)`. `matches!` in a guard takes a call on a
bound value, as `matches!(item.kind(), Kind::Write)`, since a pattern
can't call a method. A `filter` of one comparison on an existing `Option`
whose result is matched at once is a guard as well.

When an arm needs several equalities, bind a tuple of each side in the
scrutinee and compare it in one guard, as `crates/agents/src/bin/s1.rs`
does with `seen if seen == (&expected, &t.text)`. Equality is never `cmp`
against `Ordering::Equal`; `cmp` is for ordering (§3).

## 26. Awaited effects in order are a `for`

```rust
pub async fn replay(log: &Log, session: SessionId, kinds: Vec<EventKind>) -> Result<(), Stopped> {
    for kind in kinds {
        log.append(session, kind).await?;
    }
    Ok(())
}
```

A loop whose body only performs side effects is a `for` (proposal 0041).
Its body holds only calls: `f(item);`, `f(item)?;`, `x.m(item).await?;`,
or `let _ = f(item);` to discard a `#[must_use]` result (§19). An `.await`
is always followed by `?`. It produces no value: building or selecting
values inside a `for`, in a call's arguments or in its iterator
expression, is a violation, and stays the adapters' job (§8). Inside an actor, the body calls a `&mut self`
method, as in `self.step(step).await?;` in
`crates/daemon/src/session.rs`. `for_each`, `try_for_each` and
`stream::iter(..).then(..).try_collect()` into `()` are no longer the
form for running effects. A `then` chain stays for building a list from
awaited steps, with the list's type in `try_collect`'s turbofish.

## 27. A table scanned in order may hold strings

```rust
struct Row {
    marker: &'static str,
    name: &'static str,
    touch_name: &'static str,
}

/// Checked in this order: an iPhone's user agent also says "Mac OS X".
const DEVICES: &[Row] = &[
    Row { marker: "iPhone", name: "iPhone", touch_name: "iPhone" },
    Row { marker: "Macintosh", name: "Mac", touch_name: "iPad" },
];

pub fn browser_name(user_agent: &str, touch: Touch) -> ClientName {
    let name = DEVICES
        .iter()
        .find(|row| user_agent.contains(row.marker))
        .map_or("Browser", |row| match touch {
            Touch::None => row.name,
            Touch::Screen => row.touch_name,
        });
    ClientName(String::from(name))
}

pub fn os_name(os: String) -> ClientName {
    ClientName(match os.as_str() {
        "windows" => String::from("Windows"),
        "macos" => String::from("Mac"),
        _ => os,
    })
}
```

A `const` table that is scanned in order, with one predicate that tests
the input against a field, may have `&'static str` fields (proposal 0042).
The row struct is built only inside the `const`, and the order of the rows
is the rule: here the first marker the user agent contains wins. A lookup
keyed by a string is a `match`, as in `os_name`.
A predicate that compares a row's string for equality with a value, such
as `DEVICES.iter().find(|row| row.name == name)`, is flagged in any
spelling (`&&`, `!(.. != ..)`, `.eq`, `any`, `enumerate`), and so is
`&'static str` anywhere but a row struct's field: in a signature or in a
struct that isn't only a table's row. A row struct is built only as an
element of its table. A `const` table of tuples holding a `&str`, with the
lifetime spelled or left out, as in `const PAIRS: &[(&str, Kind)]`, is
flagged too: its row is a named struct.

## 28. Counting through numbers is `RangeInclusive::new`

```rust
use std::ops::RangeInclusive;

pub fn candidates(first: u16, last: u16) -> Vec<u16> {
    RangeInclusive::new(first, last).collect()
}
```

`first..=last` is `expr.range`, which stays out. A range built by name
stands only where it is used at once: as a `for`'s iterator, the receiver
of an iterator method, the receiver of `contains` (a membership test), or
the argument of `zip` or `chain`.
`bytes.get(RangeInclusive::new(1, 2))` is indexing by range under another
name and is flagged, as are `RangeInclusive::new(1, 2).into_iter()` in the
same place, `Range::default()`, `Range { start, end }`, `RangeFull` and
any `Bound`. Split bytes with `split_at_checked` (§A7 of the bootstrap
review); `split_at` panics and is flagged.

## 29. A helper used in one block can be a local closure

```rust
struct Entry {
    label: String,
}

fn entries(prefix: &str, first: Vec<String>, second: Vec<String>) -> (Vec<Entry>, Vec<Entry>) {
    let entry = |label: String| Entry { label: format!("{prefix}{label}") };
    (
        first.into_iter().map(&entry).collect(),
        second.into_iter().map(entry).collect(),
    )
}
```

Proposal 0046 allows an immutable `let` closure when its uses are only
calls in the same block, including nested synchronous blocks. Call it
directly, or pass it by value or shared reference to a call that already
admits closures. A plain parameter may have a type annotation when
inference needs it. Every other construct in its body keeps its rule.

The closure is never unused, returned, aliased, stored, or captured by
another closure or async block. Predicates containing boolean expressions
and the permitted async closure bodies are still written at the adapter
call. A helper shared across functions stays a free function.

## 30. Literal arrays can receive methods

```rust
fn joined(first: String, second: String) -> String {
    [first, second].concat()
}

fn labels() -> Vec<String> {
    ["Claude", "Codex"].map(String::from).to_vec()
}

fn count() -> usize {
    (&["Claude", "Codex"]).len()
}
```

Proposal 0047 keeps the literal in its expression, including shared
references and grouping. A local array binding or returned array literal
remains separate from this rule; byte-array types keep 0015's limits.

## 31. Keep the whole value selected by a pattern

```rust
enum Event {
    Open,
    Updated,
    Closed,
}

fn live(event: Event) -> Option<Event> {
    match event {
        event @ (Event::Open | Event::Updated) => Some(event),
        Event::Closed => None,
    }
}

fn selected(value: u64) -> Option<u64> {
    match value {
        n @ 3 => Some(n),
        n @ (1 | 2) => Some(n),
        _ => None,
    }
}

fn pair(values: (u64, u64)) -> u64 {
    match values {
        pair @ (a, b) => pair.0.saturating_add(a).saturating_add(b),
    }
}

fn whole(value: String) -> String {
    match value {
        whole @ _ => whole,
    }
}
```

Proposal 0048 allows `@` around every otherwise-permitted pattern. The
outer binding is immutable and has no `ref`. Parentheses group the inner
pattern. Rust still checks ownership: destructuring copied tuple fields
works, while moving owned `String` fields and retaining their tuple does
not. Integer/character range patterns also work under 0049. Boolean
patterns and explicit reference patterns keep their existing restrictions.

## 32. Sequence patterns show lengths and payloads

```rust
fn exact(bytes: &[u8]) -> Option<Vec<u8>> {
    match bytes {
        [first, second] => Some(Vec::from([first.to_owned(), second.to_owned()])),
        _ => None,
    }
}

fn payload(bytes: &[u8]) -> Option<Vec<u8>> {
    match bytes {
        [0x01, remaining @ ..] => Some(remaining.to_vec()),
        _ => None,
    }
}

fn middle(bytes: &[u8]) -> Option<Vec<u8>> {
    match bytes {
        [first, middle @ .., last] => Some(middle.to_vec()),
        _ => None,
    }
}

fn packet(bytes: &[u8]) -> Option<Vec<u8>> {
    match bytes {
        bytes @ ([0x01, _, _] | [0x02, _, _]) => Some(bytes.to_vec()),
        _ => None,
    }
}

fn all_lengths(bytes: &[u8]) -> Vec<u8> {
    match bytes {
        [] => Vec::new(),
        [first, ..] => Vec::from([first.to_owned()]),
    }
}

fn fixed(bytes: [u8; 2]) -> Vec<u8> {
    let [first, second] = bytes;
    Vec::from([first, second])
}
```

Proposal 0047 admits array/slice patterns and a rest directly within them,
optionally bound by immutable `@`. The fallback covers other lengths and
values; `[]` and `[first, ..]` instead partition every possible length.
Rust rejects a match that leaves lengths uncovered, and a guarded arm
needs an unguarded covering fallback. Version 14 also admits unbound tuple
and tuple-struct rests and shared slice outputs (examples below).

## 33. Intervals select named states directly

```rust
enum AttemptState {
    Fresh,
    Retrying,
    Exhausted,
}

fn classify(attempts: u8) -> AttemptState {
    match attempts {
        0 => AttemptState::Fresh,
        1..=3 => AttemptState::Retrying,
        4.. => AttemptState::Exhausted,
    }
}

fn signed(value: i16) -> Option<i16> {
    match value {
        ..=-1 => None,
        n @ (0..=3) => Some(n),
        4.. => None,
    }
}

fn letter(value: char) -> Option<char> {
    match value {
        letter @ ('a'..='z' | 'A'..='Z') => Some(letter),
        _ => None,
    }
}

fn guarded(value: u8, expected: u8) -> Option<u8> {
    match value {
        n @ (1..=3) if n == expected => Some(n),
        0..=4 => None,
        _ => None,
    }
}

fn full(value: u8) -> u8 {
    let n @ 0..=255 = value;
    n
}
```

Proposal 0049 allows integer, byte, and character ranges, including
inclusive `..=`, exclusive `..`, missing endpoints, grouping, and nesting.
A signed match needs negative cases. A binding or parameter must cover
its full input type. A guarded range needs a later unguarded covering
pattern; identical symbolic bounds are recognized, while literal bounds
also support containment. Float/boolean ranges, range expressions, and
general comparisons keep their existing rules. Constant endpoint types
remain a review check.

## 34. Cardinality belongs in an immutable model

```rust
use std::time::Duration;

struct RetrySchedule { delays: [Duration; 4] }
fn schedule() -> RetrySchedule {
    RetrySchedule {
        delays: [Duration::ZERO, Duration::from_secs(1), Duration::from_secs(2), Duration::from_secs(4)],
    }
}
enum Provider { Claude, Codex }
const COUNT: usize = 2;
fn providers() -> [Provider; COUNT] {
    let providers = [Provider::Claude, Provider::Codex];
    providers
}
fn labels(first: String, second: String) -> [String; 2] { [first, second] }
```

0050 admits any otherwise-permitted element type. Rust checks cardinality
and ownership. Use a fixed array for a meaningful size, and a slice for an
ordinary constant list. Mutation, indexing and computed length expressions
keep their existing restrictions.

## 35. Return a shared view

```rust
fn payload(bytes: &[u8]) -> Option<&[u8]> {
    match bytes {
        [0x01, remaining @ ..] => Some(remaining),
        _ => None,
    }
}
struct Packet { bytes: Vec<u8> }
impl Packet {
    fn bytes(&self) -> &[u8] { self.bytes.as_slice() }
}
```

0051 adds shared outputs, including `Option<&[T]>` and `Result<&[T], E>`.
Rust's lifetime elision ties the output to the input or `self`. A dangling
view or ambiguous input borrow fails compilation. Stored slice fields,
mutable outputs and explicit domain lifetimes remain outside.

## 36. Preserve the identity type through a shared shape

```rust
struct Step<Id> { provider: Id, request: String }
enum Decision<Id> { Approved(Step<Id>), Rejected(Id) }
impl<Id> Step<Id> {
    fn new(provider: Id, request: String) -> Self { Self { provider, request } }
    fn with_request(self, request: String) -> Self { Self { request, ..self } }
}
fn keep<Id>(step: Step<Id>) -> Step<Id> { step }
fn view<T>(values: &[T]) -> Option<&[T]> {
    match values { [_, remaining @ ..] => Some(remaining), [] => None }
}
```

0052 allows unbounded type parameters on structs, enums, inherent impls
and functions, without defaults or `where`. It preserves different id
types without converting them to strings. Codec bounds retain their
existing exception; general bounds, const generics, generic aliases and
user-defined traits remain outside.

## 37. Match borrowed structure and omit unused fields

```rust
fn borrowed(value: &Option<u8>) -> Option<u8> {
    match value { &Some(code) => Some(code), &None => None }
}
fn first(value: (u8, u8, u8)) -> u8 {
    let (first, ..) = value;
    first
}
fn ends(value: (u8, u8, u8), expected: u8) -> Option<u8> {
    match value {
        (first, .., last) if first == expected => Some(last),
        (_, ..) => None,
    }
}
fn keep(value: Option<String>) -> Option<String> {
    match value { whole @ Some(..) => whole, None => None }
}
```

0053 admits shared `&Pattern` and unbound tuple/tuple-struct rests, including
`(.., last)` and `Some(..)`. Only slice rests may bind an immutable name
with `@`. Rust rejects moves from borrowed owners; `&mut`, `ref`, bound
tuple rests and expression dereferencing remain outside.

## 38. Convert missing fields and flags at the wire boundary

```rust
// crates/agents/src/codex/protocol.rs
use serde::Deserialize;

#[derive(Deserialize)]
struct ModelRow {
    #[serde(default)]
    hidden: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    effort: Option<String>,
}
enum Visibility { Visible, Hidden }
fn visibility(hidden: Option<bool>) -> Visibility {
    match hidden {
        Some(true) => Visibility::Hidden,
        Some(false) | None => Visibility::Visible,
    }
}
#[derive(Deserialize)]
#[serde(tag = "kind", content = "payload")]
enum Event { Known(String), #[serde(other)] Other }
```

0054 confines boolean patterns to wire modules and requires conversion
to named domain states by review. 0055 permits plain field defaults and
unit fallback variants in tagged wire enums. A missing boolean defaults
to false; null is still an error. An optional field can retain absence.
Review must check that defaults preserve the protocol's meaning and that
discarding an unknown event's payload is appropriate.

## 39. Embed text and retain typed error context

```rust
const PINS: &str = include_str!("pins.toml");

#[derive(Debug, thiserror::Error)]
enum LoadError {
    #[error("loading {path}")]
    Io { path: String, #[source] cause: std::io::Error },
}
```

0056 permits one literal `include_str!` path in a const initializer; Rust
checks the file and rebuilds when it changes. 0057 permits a plain source
field on a derived error, preserving the original cause alongside context.
At most one causal field is permitted per error struct or enum variant.

## Still outside the subset

- Generic aliases, user-defined traits, defaults, `where`, const generics
  and any new bound beyond the existing codec/library-trait exceptions.
- `Display`, `FromStr` and `From` impls. Write `label`, `parse` and `to_…`
  methods instead (proposal 0009).
- Derives beyond the listed ones, such as `Default`, and `PartialOrd` or
  `Ord` on anything but an ordered newtype.
- `if`, `if let` with an `else` or a let-chain, `while`, a `for` that
  builds or selects values, a guard with `&&`, `||` or `!`, `return` and the
  operators. They stay rewrites into the core
  (`docs/bootstrap-review.md`).
