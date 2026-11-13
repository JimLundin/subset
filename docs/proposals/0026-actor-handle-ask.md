---
construct: generics.type_param, type.impl_trait.arg, expr.closure (one request helper per actor handle or crate)
status: accepted
task: rewrite of crates/agents (rw-agents, friction rows C4, Y1)
---

# One `ask` helper per actor handle or crate

## Rule
An actor's handle may have one private helper that does the request and reply round trip. It is written in exactly this shape:

```rust
async fn ask<T>(&self, build: impl FnOnce(oneshot::Sender<T>) -> Msg) -> Result<T, E> {
    let (reply, answer) = oneshot::channel();
    self.commands.send(build(reply)).await.map_err(|_| E::…)?;
    answer.await.map_err(|_| E::…)
}
```

Each handle method that waits for a reply calls it with a closure that only builds the message variant, as in `self.ask(|reply| Msg::Shutdown { reply })`. The type parameter has no bound, the `impl` argument is only `FnOnce(oneshot::Sender<T>) -> Msg`, and the closure is allowed only as `ask`'s argument. A handle with a free-function helper (codex's `CodexServer` and `CodexThread` share one command sender) takes the sender as the first parameter instead of `&self`. Nothing else gets a type parameter outside 0013, or an `impl Trait` argument.

This revisits the `ask<T>`/`request<T>` part of the bootstrap review's §A11. §A11 measured only `crates/daemon/src/host.rs`, where the helper saved 9%. The agents rewrite adds two more handles, and across all three the saving is 16.6%.

Amended in version 10 by [issue #3](https://github.com/JimLundin/subset/issues/3):
a crate with multiple actor handles that all use one stopped-error type
has one shared free function instead of the per-handle methods:

```rust
pub(crate) async fn ask<M, T>(
    inbox: &mpsc::Sender<M>,
    build: impl FnOnce(oneshot::Sender<T>) -> M,
) -> Result<T, Stopped> {
    let (reply, answer) = oneshot::channel();
    inbox.send(build(reply)).await.map_err(|_| Stopped {})?;
    answer.await.map_err(|_| Stopped {})
}
```

It may be private or `pub(crate)`. Both type parameters have no bounds or
defaults, with no `where`. `M` appears only as the sender's item and
`build`'s return type. A handle calls it as
`ask(&self.inbox, |reply| Msg::Variant { .., reply })`. A crate with one
handle, or with handles that fail differently, keeps the original rule.
Whether all handles share one error type, use one helper and build only
message variants remains a review check. The amendment's submitted diff,
measurements and limitations are retained below.

## Before
`crates/agents/src/claude/session.rs:171-224` as rewritten. Each method builds and awaits its own `oneshot`, through private `send` and `ask_control` helpers. Codex's `server.rs:167-187` and `227-243` has the same shape with free `request`, `ask` and `send` functions. The host's three methods are §A11's before.

```rust
pub async fn send_user(&self, text: String) -> Result<(), Error> {
    let (reply, answer) = oneshot::channel();
    self.send(CommandMsg::SendUser { text, reply }).await?;
    answer.await.map_err(|_| Error::Exited)?
}

pub async fn shutdown(self) -> Result<ExitStatus, Error> {
    let (reply, answer) = oneshot::channel();
    self.send(CommandMsg::Shutdown { reply }).await?;
    answer.await.map_err(|_| Error::Exited)
}

async fn control(&self, call: ControlCall) -> Result<Value, Error> {
    tokio::time::timeout(CONTROL_TIMEOUT, self.ask_control(call))
        .await
        .map_err(|_| Error::Timeout(String::from(call.subtype())))?
}

async fn ask_control(&self, call: ControlCall) -> Result<Value, Error> {
    let (reply, answer) = oneshot::channel();
    self.send(CommandMsg::Control { call, reply }).await?;
    answer.await.map_err(|_| Error::Exited)?
}

async fn send(&self, msg: CommandMsg) -> Result<(), Error> {
    self.commands.send(msg).await.map_err(|_| Error::Exited)
}
```

## After
```rust
pub async fn send_user(&self, text: String) -> Result<(), Error> {
    self.ask(|reply| CommandMsg::SendUser { text, reply }).await?
}

pub async fn shutdown(self) -> Result<ExitStatus, Error> {
    self.ask(|reply| CommandMsg::Shutdown { reply }).await
}

async fn control(&self, call: ControlCall) -> Result<Value, Error> {
    tokio::time::timeout(CONTROL_TIMEOUT, self.ask(|reply| CommandMsg::Control { call, reply }))
        .await
        .map_err(|_| Error::Timeout(String::from(call.subtype())))??
}

async fn ask<T>(&self, build: impl FnOnce(oneshot::Sender<T>) -> CommandMsg) -> Result<T, Error> {
    let (reply, answer) = oneshot::channel();
    self.commands.send(build(reply)).await.map_err(|_| Error::Exited)?;
    answer.await.map_err(|_| Error::Exited)
}
```

The full before and after for all three handles are `docs/subset/measurements/0026/before.rs` and `after.rs`. The two agents rewrites were applied to a scratch worktree. They compile, pass `cargo clippy --all-targets -- -D warnings` and pass all 31 `agents` tests, including both replay suites, which drive every handle method against recorded CLI output. `subset check` then reports `generics.type_param` and `type.impl_trait.arg` on each `ask`, and `expr.closure` on the three codex call sites, which are free-function calls. It doesn't flag the closures passed to `self.ask`, because it can't tell that `ask` is our method (see the review's tool notes).

## Measurements
| | tokens | lines |
|---|---|---|
| before | 932 | 94 |
| after | 777 | 92 |
| change | -155 (-16.6%) | -2 |

By handle: claude `ClaudeSession` 408 to 332 (-18.6%), codex `CodexServer`/`CodexThread` 314 to 253 (-19.4%), daemon `Host` 210 to 192 (-8.6%, §A11's numbers).

Friction points removed: 2 (C4, Y1), plus §A11's host.rs site.

## Qualities
1. Named states: kept. The message is still a named variant, built in the caller.
2. Visible branching: kept. There is no branching in the helper. `control` and `request` end in `??`, two error exits in a row (timeout, then the actor's answer), each still a `?`.
3. Forward data flow: kept.
4. Errors are values: kept. The same errors pass through the same `map_err`s.
5. Local reasoning: kept. `ask` sits in the same `impl` as its callers and is five lines. The closure only builds a variant, and `T` is the type of that variant's `reply` field.
6. One way: helped. Today the crate has two shapes for the round trip (claude's methods with `send` and `ask_control`, codex's free `request`, `ask` and `send`), and each handle method repeats the `oneshot` steps. After, every round trip is one `ask` call.

## Owner decision
Accepted on 2026-11-11, in subset version 4, as narrowly as the syntax allows. `type.impl_trait.arg` joins the subset only for the `build` parameter of a private fn named `ask`, written as `impl FnOnce(oneshot::Sender<T>) -> Msg`, where `T` is the fn's one type parameter, with no bound and no `where`. A closure outside a method call is allowed as an argument of `ask`. That every round trip goes through `ask`, and that the closure only builds the variant, stay review checks.


The owner requested implementation of issue #3 on 2026-11-13. Its amendment
to 0026 is implemented in subset version 10. The existing tolerated tiers
for the generics and builder argument remain; boundary use does not apply.

## Amendment evidence (issue #3)

The following is the submitted candidate at the stated Conductor commit.
Its measurements and validation describe that submission; the Conductor
rewrite and daemon tests were not rerun as part of this checker change.

Written to the draft guide in #2.

- **Project and commit:** JimLundin/conductor at `84b6342`.
- **Subset:** revision `ece845d`, `subset.toml` version 8.
- **Reopens:** proposal 0026, "One `ask` helper per actor handle". 0026 measured three handles (16.6%). Conductor now has seven, four of them in one crate with the same error type, which 0026 did not have in front of it.

### Friction

`crates/daemon` has four actor handles. Each carries its own copy of the helper, identical except for the message type:

| Site | Message type |
|---|---|
| `crates/daemon/src/log.rs:84` | `LogMsg` |
| `crates/daemon/src/registry.rs:161` | `RegistryMsg` |
| `crates/daemon/src/session.rs:184` | `SessionMsg` |
| `crates/daemon/src/codex_hub.rs:118` | `HubMsg` |

Found with `grep -rn "async fn ask<" crates`, which prints seven lines: these four, `crates/agents/src/claude/session.rs:234`, `crates/agents/src/codex/server.rs:236` and `crates/client/src/link.rs:217`.

**Wrote** (`registry.rs:161`; the other three differ only in the message type):

```rust
async fn ask<T>(
    &self,
    build: impl FnOnce(oneshot::Sender<T>) -> RegistryMsg,
) -> Result<T, Stopped> {
    let (reply, answer) = oneshot::channel();
    self.inbox
        .send(build(reply))
        .await
        .map_err(|_| Stopped {})?;
    answer.await.map_err(|_| Stopped {})
}
```

**Wanted:** the helper once, in `crates/daemon/src/lib.rs` beside `Stopped`.

**Why not:** 0026 allows `T` as the only type parameter and `build`'s return type as a concrete `Msg`. Sharing the helper needs a second unbounded parameter for the message type.

### Rule

A crate whose actor handles all fail with one stopped-error type has one `ask`, a free function in the 0026 free-function shape with the message type as a second unbounded parameter:

```rust
pub(crate) async fn ask<M, T>(
    inbox: &mpsc::Sender<M>,
    build: impl FnOnce(oneshot::Sender<T>) -> M,
) -> Result<T, Stopped> {
    let (reply, answer) = oneshot::channel();
    inbox.send(build(reply)).await.map_err(|_| Stopped {})?;
    answer.await.map_err(|_| Stopped {})
}
```

A handle calls it as `ask(&self.inbox, |reply| Msg::Variant { .., reply })`. In such a crate the per-handle method form goes. `M` has no bound and appears only as the sender's item and `build`'s return type. A crate with one handle, or with handles that fail differently, keeps 0026 as it is.

### Before and after

Applied to a scratch worktree of `84b6342`. `cargo fmt` and `cargo clippy -p daemon --all-targets -- -D warnings` pass. The daemon's tests were not run.

<details>
<summary>The diff (5 files, 33 insertions, 59 deletions)</summary>

```diff
--- a/crates/daemon/src/lib.rs
+++ b/crates/daemon/src/lib.rs
@@ -16,12 +16,23 @@
 use proto::{AuthToken, Epoch};
+use tokio::sync::{mpsc, oneshot};

 /// The actor stopped: nobody can reach it any more.
 #[derive(Debug, thiserror::Error)]
 #[error("the actor has stopped")]
 pub struct Stopped {}

+/// Sends an actor the message `build` makes and waits for its reply.
+pub(crate) async fn ask<M, T>(
+    inbox: &mpsc::Sender<M>,
+    build: impl FnOnce(oneshot::Sender<T>) -> M,
+) -> Result<T, Stopped> {
+    let (reply, answer) = oneshot::channel();
+    inbox.send(build(reply)).await.map_err(|_| Stopped {})?;
+    answer.await.map_err(|_| Stopped {})
+}
+
--- a/crates/daemon/src/log.rs
+++ b/crates/daemon/src/log.rs
@@ -74,20 +74,11 @@ impl Log {
     pub async fn welcome(&self, providers: Providers) -> Result<Welcome, Stopped> {
-        self.ask(|reply| LogMsg::Welcome { providers, reply }).await
+        ask(&self.inbox, |reply| LogMsg::Welcome { providers, reply }).await
     }

     pub async fn subscribe(&self, after: HostSeq) -> Result<Subscription, Stopped> {
-        self.ask(|reply| LogMsg::Subscribe { after, reply }).await
-    }
-
-    async fn ask<T>(&self, build: impl FnOnce(oneshot::Sender<T>) -> LogMsg) -> Result<T, Stopped> {
-        let (reply, answer) = oneshot::channel();
-        self.inbox
-            .send(build(reply))
-            .await
-            .map_err(|_| Stopped {})?;
-        answer.await.map_err(|_| Stopped {})
+        ask(&self.inbox, |reply| LogMsg::Subscribe { after, reply }).await
     }
 }
--- a/crates/daemon/src/registry.rs
+++ b/crates/daemon/src/registry.rs
@@ -138,36 +138,30 @@ impl Registry {
     pub async fn find(&self, session: SessionId) -> Result<Option<Session>, Stopped> {
-        self.ask(|reply| RegistryMsg::Find { session, reply }).await
+        ask(&self.inbox, |reply| RegistryMsg::Find { session, reply }).await
     }

     pub async fn set_credential(&self, credential: Credential) -> Result<CommandResult, Stopped> {
-        self.ask(|reply| RegistryMsg::SetCredential { credential, reply })
-            .await
+        ask(&self.inbox, |reply| RegistryMsg::SetCredential {
+            credential,
+            reply,
+        })
+        .await
     }
```

`session.rs` and `codex_hub.rs` change the same way: the method goes and each `self.ask(` becomes `ask(&self.inbox, `. Each file's `use crate::Stopped;` becomes `use crate::{Stopped, ask};`.

</details>

One site gets worse: `set_credential` and `clear_credential` in `registry.rs` go from two lines to five each, because the longer call no longer fits rustfmt's width.

### `subset check` on the after

```
crates/daemon/src/lib.rs:27:25: generics.type_param: a type parameter only on a fn, with exactly one bound, `Serialize` or `DeserializeOwned`, and no `where` (proposal 0013), or as the unbounded `T` of an `ask` helper (proposal 0026)
crates/daemon/src/lib.rs:27:28: generics.type_param: a type parameter only on a fn, with exactly one bound, `Serialize` or `DeserializeOwned`, and no `where` (proposal 0013), or as the unbounded `T` of an `ask` helper (proposal 0026)
crates/daemon/src/lib.rs:29:12: type.impl_trait.arg: `impl Trait` as an argument only as `build: impl FnOnce(oneshot::Sender<T>) -> Msg`, the last parameter of a private `ask<T>` helper (proposal 0026)
```

### Weight

tolerated, as 0026's parts are today (`generics.type_param`, `type.impl_trait.arg`).
Boundary use: doesn't apply.

### Measurements

`subset score --by file`, run from the Conductor root with its `subset.toml`.

The helpers alone (the four `ask` methods before, the one function after):

| | score | tokens | lines |
|---|---|---|---|
| before | 366.0 | 326 | 46 |
| after | 100.5 | 90 | 8 |
| change | -265.5 (-72.5%) | -236 (-72.4%) | -38 |

The five whole files, which include the ten call sites and the imports:

| | score | tokens | lines |
|---|---|---|---|
| before | 9328.0 | 9779 | 1446 |
| after | 9150.5 | 9611 | 1422 |
| change | -177.5 (-1.9%) | -168 (-1.7%) | -24 |

The after scores use the proposed tier. `subset score` printed 121.5 and 9171.5, pricing the three flagged constructs as outside: `type.impl_trait.arg` (5 tokens, 25.0) and two `generics.type_param` (2 tokens, 10.0). At tolerated (2.0) they cost 10.0 and 4.0, which takes 21.0 off both totals.

The call sites cost what the helpers save in part: the helpers drop 236 tokens and the files drop 168, so the ten call sites and five imports add 68.

**Which test it meets:** four friction points of one kind. The helper code alone drops 72.5%; the files as a whole drop 1.9%, since the helpers are a small share of them.

### Qualities

1. Named states: kept. Nothing about states changes.
2. Visible branching: kept. The helper has no branch before or after.
3. Forward data flow: kept.
4. Errors are values: kept. The same `Result<T, Stopped>`.
5. Local reasoning: kept, with a cost. A handle method now reads `ask(&self.inbox, ..)` and the helper is in `lib.rs`, one file away instead of at the bottom of the same `impl`. The shape is still fixed by rule, so a reader who knows one knows all.
6. One way: kept, and helped within such a crate. 0026 already has two forms (the method, and the free function for `CodexServer` and `CodexThread`). A crate under this rule has only the free function.
7. Repetition is data: helped. Four places share a shape and differ only in a type, and the shape is written once.

### Tool change

`subset check` allows a second unbounded type parameter on an `ask` helper when it is a free function whose first parameter is `&mpsc::Sender<M>` and whose `build` returns `M`. `pub(crate)` is allowed on that function, where 0026 requires private. Whether to flag a crate that keeps per-handle helpers though all share one error type is a review check or a checker rule, at the owner's choice.

### Not measured

- `crates/agents`: its two helpers fail with two different `Error::Exited`s, one per adapter module, so sharing one needs a common stopped type first.
- `crates/client`: one handle, so the rule doesn't apply.
- The daemon's tests on the rewrite.

Related: JimLundin/conductor#4.
