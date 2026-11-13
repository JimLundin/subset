# Conductor issue #5 combined consumer review for v15

Reviewer: independent GPT-6.1 Sol.
Worktree: `/home/jim/projects/apps/conductor-issue5-v14`.
Base: `79568ce8a55b266fe1cf96b17dd5b2bc2dfa0c8c`.
Scope: the complete current 16-file Rust diff under `crates/`, 99 insertions and 96 deletions, plus the updated `.audit/subset-issue5-v14.friction.md`.

Reviewed Rust patch SHA-256:
`04590429c69c7945e774e4a0add7e58884157e062fa1a26350bff113a7cc0d62`.

Reproduction:

```sh
git diff --no-ext-diff --no-color 79568ce8a55b266fe1cf96b17dd5b2bc2dfa0c8c -- crates | sha256sum
```

The hash was checked before and after review and remained unchanged. No implementer reasoning or private process logs were consulted. Inputs were the actual diff, friction log, Conductor `AGENTS.md` and pair skill, published version-14 policy, and proposal 0058's exact rule. The coordinator explicitly reported the owner's acceptance of 0058 and instruction that 0059 remains proposed.

## Required fixes and decision

No required Rust fixes found. Manual approval for this exact combined Rust patch: correctness, volume, and all seven subset qualities pass under the published v14 constructs plus the owner's accepted 0058 rule.

This approval excludes the upcoming v15 checker pin/config changes and is not a claim that the complete gate has passed. Those changes must be reviewed separately when available, and `tools/check.sh` remains required before committing.

## New 0058 consumer rewrite

`crates/agents/src/claude/session.rs:472` uses exactly:

```rust
std::iter::from_fn(|| events.try_recv().ok()).find_map(|event| match event {
    ClaudeEvent::TurnFinished { errors, .. } => errors.first().map(|_| errors.join("; ")),
    _ => None,
})
```

The receiver is concretely `tokio::sync::mpsc::Receiver<ClaudeEvent>`, imported from Tokio, rather than a same-spelled domain method. The generator is an inline zero-argument synchronous closure with exactly `try_recv().ok()` as its body. Its direct consumer is the eager `find_map`. Only the owned terminal result is bound; the generator and mutable receiver borrow do not escape the expression. No named callback, lazy-only adapter, blocking pull, mutable domain counter, or generalized closure signature is introduced.

Behavior remains the same:

- Empty and disconnected receiver outcomes become `None`, ending this drain immediately; it does not await new channel input.
- Other events and `TurnFinished` values with empty error lists are consumed and skipped as before.
- The first nonempty error list is joined using the same separator. A nonempty list containing an empty string still produces `Some("")`, preserving the original behavior.
- The terminal stops immediately on that first matching result. Later queued events remain untouched. `FromFn` need not be fused because this terminal stops on its first `None` and the generator is then discarded.
- The stderr fallback and its timeout remain unchanged. The `last_stderr` handle is consumed only on that fallback, as before.

I checked the actual dependency implementation: `n0_future` reexports `futures_lite::stream` and `StreamExt`; futures-lite 2.6.1 `Unfold::poll_next`, `FilterMap::poll_next`, and `NextFuture::poll` introduce no yield for this generator, whose async body had no await. Thus the removed stream await was immediately ready and did not provide an extra cancellation/scheduler boundary. The real stderr await remains the same. No new race or event-loss behavior is introduced.

Existing replay tests cover a refusal reported on stdout and fallback to the CLI's last stderr line. They were inspected but not executed during this review.

## Combined v14 changes rechecked

- `crates/daemon/src/log.rs:107`: the grouped owned pattern retains the full original assistant/reasoning delta kind and payload. Both variants still send one live event at the existing head without entering replay. Other events still increment and persist. The regression checks both deltas, live ordering, sequence values, and replay exclusion.
- `crates/agents/src/install.rs:241` and `:315`: local error helpers capture only shared paths, remain confined, and retain each original error path and `std::io::Error`. Installer locking, polling, cancellation, and filesystem effects are unchanged. No 0059 factory or escaping closure is used.
- `crates/agents/src/bin/s1.rs:409`: the local chosen-options helper owns the same output and preserves resume/log-path inputs; it captures only the shared CLI path.
- `crates/app/src/session.rs:272`: decision dispatch precedes focus-script evaluation at click time, with the same request target and no added async work.
- `crates/daemon/src/registry.rs:245` and `crates/daemon/src/session.rs:77`: reference patterns copy only the existing `Copy` enums Provider and Effort. Handle cloning, readiness errors, and effort selection are unchanged.
- `crates/proto/src/frame.rs:24` and `:38`: the endpoint is the u32 constant 16,777,216. Inclusive ranges preserve acceptance of zero and the exact maximum, rejection of maximum plus one and u32::MAX, and checked integer-conversion errors. Encoding preserves the original u64 reporting fallback.
- Array rewrites preserve owned iteration under edition 2024, left-to-right evaluation of awaited reads, concatenation order, and all test data. Multi-field case rows remain named structs. The removed temporary vectors add no needed semantics.

## Volume, friction, and qualities

The diff consolidates existing repeated shapes and removes wrappers; it adds no speculative options, extra variants, public API, or duplicate abstraction. The updated friction log removes the resolved synchronous-drain entry and correctly keeps actor setup, blocking-worker, and escaping-error-factory limitations. 0059 remains proposed and absent from the consumer implementation.

Named states, explicit branching, immutable domain data flow, Result/Option error handling, local reasoning, a single idiom for each task, and data-based repetition are preserved. The 0058 external mutable borrow stays within the accepted library boundary and immediate synchronous expression.

## Pending publication and verification scope

At review time the consumer still had config version 14 and its old checker pin. In the Windows subset checkout, `main` at `de8da88c80f20909bd1a3758be0f6de5c1fe46c0` also still labelled proposal 0058 as proposed with owner decision pending. This review relies on the coordinator's explicit statement of owner acceptance; publication of that acceptance and the exact v15 rule/pin/config must be checked in the separate follow-up review. This stale text does not change the Rust assessment.

No builds, compilation, cargo invocation, or tests were started by the reviewer. Read-only Linux git confirmed that the earlier native UNC observation of a deleted `CLAUDE.md` symlink was an artifact: actual Linux status lists only the 16 changed Rust files.
