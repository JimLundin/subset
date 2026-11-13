---
construct: expr.closure
status: accepted
task: issue #5, synchronous Conductor channel draining
---

# Drain a synchronous library receiver with a pull iterator

## Rule

An inline zero-argument synchronous closure may be the direct argument of
`std::iter::from_fn` to pull available values without blocking from an
external receiver whose existing API requires `&mut`. Its body is exactly
the existing `receiver.try_recv().ok()` call chain; `None` explicitly ends
the drain. The `from_fn` call is the direct receiver of the eager terminal
`find_map` in that same expression, using its already permitted inline
adapter closure. Only the terminal result may be bound or returned.

The generator is never bound, stored, returned, aliased, passed to another
function, or captured by other work. A named callback passed to `from_fn`
is excluded. Lazy adapters such as `map`, `filter` or `take` without an eager
terminal do not satisfy the rule. Other terminals and receiver methods need
their own evidence; blocking pulls such as `blocking_recv` are excluded.

This is the one form for draining such a synchronous source. A real awaited
source keeps its existing async stream idiom. Existing 0008 mutation,
predicate, branching, error and ownership rules remain in force. The rule
does not permit mutable domain counters, `FnMut` signatures, general closure
return types, or retained generators. `std::iter::from_fn` is a specific
callee, not permission for any free function with that final segment.

## Before and after

Source: Conductor
[`79568ce8a55b266fe1cf96b17dd5b2bc2dfa0c8c`](https://github.com/JimLundin/conductor/tree/79568ce8a55b266fe1cf96b17dd5b2bc2dfa0c8c),
`crates/agents/src/claude/session.rs:468`. Subset revision
`64352fdd5935de3954ee06b4598789cb89f67a03`, config version 14.
The complete [diff](../reviews/issue5-callbacks-actors/from-fn.diff),
whole source and [measurements](../reviews/issue5-callbacks-actors/measurements.json)
are retained. No import or call site changes.

Before:

```rust
async fn refusal(
    events: &mut mpsc::Receiver<ClaudeEvent>,
    last_stderr: tokio::task::JoinHandle<Option<String>>,
) -> String {
    let errors = Box::pin(n0_future::stream::unfold(events, |events| async move {
        events.try_recv().ok().map(|event| (event, events))
    }))
    .filter_map(|event| match event {
        ClaudeEvent::TurnFinished { errors, .. } => errors.first().map(|_| errors.join("; ")),
        _ => None,
    })
    .next()
    .await;
    match errors {
        Some(errors) => errors,
        None => stderr_reason(last_stderr).await,
    }
}
```

After:

```rust
async fn refusal(
    events: &mut mpsc::Receiver<ClaudeEvent>,
    last_stderr: tokio::task::JoinHandle<Option<String>>,
) -> String {
    let errors = std::iter::from_fn(|| events.try_recv().ok()).find_map(|event| match event {
        ClaudeEvent::TurnFinished { errors, .. } => errors.first().map(|_| errors.join("; ")),
        _ => None,
    });
    match errors {
        Some(errors) => errors,
        None => stderr_reason(last_stderr).await,
    }
}
```

`log_stderr` in the same file genuinely awaits `next_line`, so its `unfold`
stream stays. It is not counted as another synchronous-drain site.

## Weight

**Tolerated, 2.0**, retaining the existing `expr.closure` tier. The closure
captures the existing receiver borrow, and its pull operation is visible
in one expression. Boundary use **applies**: this is a direct argument of
the library's `from_fn`, whose API fixes its callable shape. `library_value`
marks the `CallArg` as `Fixed::IfRestricted`; once no longer violating,
`score::judge` therefore prices its two delimiter tokens at boundary **1.0**.
This differs from 0046's local closure declaration, which is not a library
argument. Library receiver calls and signatures retain their normal prices.

## Measurements

| Complete affected function | Score | Tokens | Code lines |
|---|---|---|---|
| Before | 131.0 | 143 | 18 |
| After, outside-priced | 108.5 | 117 | 13 |
| After, tolerated with boundary use | **100.5** | **117** | **13** |
| Change, admitted | **-30.5 (-23.3%)** | **-26 (-18.2%)** | **-5 (-27.8%)** |

At version 14, `subset score --by construct` charges exactly **two** tokens to the one
outside `expr.closure`, score 10.0. The arithmetic is
`108.5 - (5.0 - 1.0) * 2 = 100.5`. No other after construct is outside.
The outside-priced result already saves **17.2%**, independently clearing
the 15% threshold.

Whole file: before **4206.0 / 4363 / 715**, after raw
**4183.5 / 4337 / 710**, after admission **4175.5 / 4337 / 710**.
The complete function is the affected code; there are no changed helpers,
imports or callers omitted from it.

Friction points removed: **one** (`refusal`). Qualification uses the score
test, not the count test. Recount at the exact commit:

```sh
git -C /path/to/conductor grep -n -E 'try_recv|from_fn|spawn_blocking' \
  79568ce8a55b266fe1cf96b17dd5b2bc2dfa0c8c -- crates
```

Reproduction commands and exact stdout/stderr/exit records are in the
[review](../reviews/issue5-callbacks-actors.md#reproduction-and-measurement-scope)
and its [script](../reviews/issue5-callbacks-actors/measure.py).
At version 14, on the after, `subset check` exits 1 and prints exactly:

```text
crates/agents/src/claude/session.rs:472:37: expr.closure: a closure outside a method call only as the argument of `Closure::new`, `unfold`, `ask` or a Dioxus hook, or an immutable local binding used only in calls in its block, directly or as an argument of the permitted closure-taking calls; never stored, aliased, unused or captured by another closure or async block (proposals 0006, 0017, 0018, 0026, 0046)
```

## Qualities

1. **Named states: kept.** The same `ClaudeEvent` variants and explicit
   `Option` termination carry the drain's state; no boolean/sentinel is added.
2. **Visible branching: kept.** The same exhaustive event `match` and stderr
   fallback `match` remain; the already present `.ok()` is unchanged.
3. **Forward data flow: kept.** No new domain value is mutable. Pulling lends
   the same existing external receiver under 0008, and the iterator cannot
   escape this synchronous expression.
4. **Errors are values: kept.** Empty/disconnected `try_recv` outcomes already
   became `None`; refusal strings and timed stderr failure handling are unchanged.
5. **Local reasoning: kept.** Source, termination and consumer appear together;
   no new helper, retained callback or asynchronous capture separates them.
6. **One way: kept.** Synchronous receiver drains use this pull iterator;
   genuinely awaited sources keep streams. General generators are excluded.
7. **Repetition is data: kept.** One adapter chain handles every queued event;
   no repeated arms or per-event code are introduced.

## Behavior and validation

Both forms stop at the first nonempty `TurnFinished.errors`, join that list
with the same separator, leave later queued events untouched, and fall back
to the same timed stderr task on empty/disconnected input. Non-refusal events
and empty error lists are consumed and skipped. Neither form waits for new
channel input: the original async closure contains no await, so its stream
polls synchronously until a result/termination. The subsequent real stderr
await has the same cancellation point.

The actual full agents crate passes `cargo check -p agents --all-targets`
on **both before and after**. The same three
[behavioral tests](../reviews/issue5-callbacks-actors/validation-tests.rs)
pass on both: empty open and disconnected channels, skipped other events
and empty refusal lists, first nonempty refusal, and untouched later events.
Both existing startup-refusal replay tests also pass on both versions:
`a_resume_with_no_transcript_fails_with_the_clis_reason` and
`a_cli_that_exits_at_start_fails_with_its_last_stderr_line`.

The [validation script](../reviews/issue5-callbacks-actors/validate.py) uses
one build job and an isolated project target. Exact commands, exit codes
and outputs are retained in
[validation-before.json](../reviews/issue5-callbacks-actors/validation-before.json)
and [validation-after.json](../reviews/issue5-callbacks-actors/validation-after.json).
The after unit build emitted one nonfatal incremental-cache permission
note on the Windows filesystem; all checks and tests exited 0. No Endpoint
networking tests or broader Conductor gate are claimed for this proposal.

```sh
python3 docs/reviews/issue5-callbacks-actors/validate.py --phase after --jobs 1
python3 docs/reviews/issue5-callbacks-actors/validate.py --phase before --jobs 1
```

## Tool change and limits

Version 15 checks the exact `std::iter::from_fn` argument position,
requires an inline zero-argument synchronous `try_recv().ok()` body, and
verifies the complete call is the direct receiver of inline synchronous
eager `find_map`. It rejects lazy chains, generator bindings/returns/helper
arguments, named callbacks, constructor imports/renames and function-value
escapes. Parentheses and `move` retain their ordinary syntax. `from_fn`
is not added to the generic callee list, so 0046's named closures cannot
reach it. Known local `std` modules do not qualify as library callees.
Actor-state discovery, mutable bindings, predicates, and genuinely awaited
stream closures do not change. Existing `expr.closure` scoring suffices.

The [receiver-drain fixtures](../../tests/cases/receiver-drain) cover the
accepted form, parentheses, `move`, external fields and binding the terminal
result, alongside rejected body/callback/terminal shapes and generator
escapes. Separate [alias](../../tests/cases/receiver-drain-alias) and
[shadow](../../tests/cases/receiver-drain-shadow) cases protect callee identity.
The [CLI regression](../../tests/cli.rs) checks both complete retained Claude
source snapshots and asserts **131.0 / 143 / 18** before and
**100.5 / 117 / 13** after admission, including existing boundary pricing.

The checker relies on syntax and configured call names, not Rust
type resolution. It can verify call path, argument/body/consumer shape and
confinement and rejects receiver bindings whose declared or constructed
type is known to be ours. It cannot prove an unresolved receiver or field
is Tokio's library receiver, or that a same-spelled method obeys the external
`&mut` rule. Import resolution and local binding classification remain
conservative, consistent with the existing checker. Type origin,
nonblocking pull/`Option` semantics and ownership therefore remain review checks plus Rust
compilation, consistent with current 0008 boundary handling. Do not present
a final-segment whitelist as proof of library behavior.

Not measured: other crates/projects, arbitrary `FnMut` generators, stored
iterators, retained callbacks, blocking workers or actor initialization.
Those requests have separate evidence and limitations in the review.

## Owner decision

Accepted by the owner on **2026-11-13**, as version **15**. The accepted
scope is the exact synchronous receiver drain above; 0059 remains proposed.
The retained Conductor rewrite is authorized by this acceptance.
