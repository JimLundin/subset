# Issue #5: callbacks and actor setup at published Conductor main

This review uses Conductor commit
[`79568ce8a55b266fe1cf96b17dd5b2bc2dfa0c8c`](https://github.com/JimLundin/conductor/tree/79568ce8a55b266fe1cf96b17dd5b2bc2dfa0c8c),
Subset revision `64352fdd5935de3954ee06b4598789cb89f67a03`, and
`subset.toml` version **14**. The Conductor checkout ahead of this commit
was read only through `git show <commit>:<path>` and `git archive <commit>`.
The issue's partly reconstructed round logs are not treated as exact source.
New rules below remain subject to the pair review and owner decision.

## Findings

| Issue item | Result at this commit |
|---|---|
| Local closures | 0046 already admits three installer helpers; their complete affected functions save 6.5 points and 13 tokens. Existing cross-function helpers stay free functions. |
| Returned error closures | Nine retained mappings after 0046 meet the friction-count test. The separate [factory review](issue5-error-factory.md) and [proposal 0059](../proposals/0059-path-error-factory.md) assess the narrow rule and validation. |
| `std::iter::from_fn` | The complete real `refusal` function clears the score bar: 131.0 to **100.5**, down **23.3%** with the proposed tolerated tier and applicable boundary weight. Even outside-priced it drops 17.2%. One synchronous channel-drain site, with no new domain mutation. |
| Shared Dioxus callbacks | Three approval buttons can use one ordinary free function today. The measured helper plus all consumers grows 4.5 points and six tokens; no new closure rule is needed. |
| `tokio::task::spawn_blocking` | One lock-acquisition site. A measured owned-file rewrite lowers the complete affected functions by 5.4%, short of 15%; cancellation/thread-pool behavior changes need separate validation. |
| Initialization before actor loop | One startup site. Reusing the complete `Order::Start` handler before the loop preserves its model/event work in the inspected source, but saves only 1.6% over complete affected functions. |
| Multiple `let mut` actor values | No surviving exact site identified. The issue's one reconstructed entry cannot establish three sites or a same-code rewrite. External mutable API values already have 0008 forms. |

## Reproduction and measurement scope

The [script](issue5-callbacks-actors/measure.py),
[JSON](issue5-callbacks-actors/measurements.json), whole before/after source,
and six diffs are retained in `issue5-callbacks-actors/`.
`pinned/` contains source verbatim; `before/` and each after directory are
formatted with the same `rustfmt --edition 2024` for measurement.
Run from this Subset root, using a repository containing the source commit:

```sh
cargo build
python3 docs/reviews/issue5-callbacks-actors/measure.py \
  --conductor /path/to/conductor \
  --subset "$PWD/target/debug/subset" --archive
```

All mutations are inside this repository's `target/` or the report's evidence
directory. The source checkout is never checked out, formatted, built or
modified. `--no-project` refreshes only measurements/evidence. On a Windows
worktree, use `wsl.exe -d Ubuntu --exec /usr/bin/python3` with the `/mnt/c/`
script path and the existing Linux Subset executable. The recorded Subset
revision is an explicit script argument/default because Windows worktree
gitfiles cannot be followed by Linux Git.

For each phase the script records exact command/stdout/stderr/exit for
`subset score --by file --json`, `--by fn --json`, `--by construct --json`,
`subset measure --json`, and `subset check`. Before, local, and ordinary
UI-helper checks exit 0 with empty stdout/stderr. New syntax violations are
retained verbatim in the JSON. Whole-function rows include changed callers,
so a removed helper's saving is never reported without its consumers.

Whole-file measurements (score / tokens / code lines):

| Alternative | Before | After, raw | After, proposed price |
|---|---|---|---|
| Installer local helpers | 3620.0 / 3610 / 482 | 3613.5 / 3597 / 483 | Same; already permitted |
| Installer returned factory | 3620.0 / 3610 / 482 | 3611.0 / 3562 / 478 | 3587.0 / 3562 / 478 |
| Claude synchronous drain | 4206.0 / 4363 / 715 | 4183.5 / 4337 / 710 | 4175.5 / 4337 / 710 |
| UI shared free helper | 2969.0 / 3189 / 498 | 2973.5 / 3195 / 506 | Same; already permitted |
| Session actor setup | 4488.0 / 4867 / 723 | 4626.0 / 4861 / 723 | 4484.0 / 4861 / 723, restoring existing actor classification |
| Blocking installer lock | 3620.0 / 3610 / 482 | 3593.5 / 3564 / 472 | 3581.5 / 3564 / 472 |

The alternatives are independent. The returned factory, local helpers and
blocking lock columns each start from the same installer file, rather than
adding their gains together.

## Existing local helpers

At original `crates/agents/src/install.rs:235`, `unpack` maps errors against
`download` three times and `dir` twice. 0046 permits one immutable closure
for each path, passed directly to its existing `map_err` consumers. At
`:312`, `sha256` uses a third closure for its two `path` mappings.
[local.diff](issue5-callbacks-actors/local.diff) contains the full change.

| Complete affected function | Before | After |
|---|---|---|
| `unpack` | 291.5 / 305 / 38 | 285.0 / 294 / 40 |
| `sha256` | 78.5 / 76 / 8 | 78.5 / 74 / 7 |
| Both | 370.0 / 381 / 46 | 363.5 / 368 / 47 |

The gain is 1.8%, with three existing helper sites; no new policy is needed.
The affected closures are immutable, return the same typed errors and do
not change branching or ownership. Their captures and uses are in the same
block, and the existing `map_err` idiom remains the only one.

`drain` at `:295` also maps `io(to, error)` twice. Adding an immutable helper
before its loop would violate 0007's whole-body rule. It is retained as
actor-setup friction rather than smuggled into the local-closure change.
The previously measured `chosen` helper in `agents/src/bin/s1.rs` remains
a supported single-block case (0046); the `reasoned` and `thread_settings`
helpers still cross function boundaries and remain free functions.

## Returned error factory

[returned.diff](issue5-callbacks-actors/returned.diff) introduces this factory,
then changes **all 16** immediate `.map_err(|error| io(path, error))` sites:

```rust
fn io_error(path: &Path) -> impl FnOnce(std::io::Error) -> InstallError {
    move |error| io(path, error)
}
```

The paths vary, including `parent`, `&lock_path`, `dir`, `download`, `binary`,
`&staging`, `to` and `path`. The existing `io(path, error)` implementation
and the `hold` branch that calls it directly are retained. Count the sites
in the exact before source with:

```sh
rg -n '\.map_err\(\|error\| io\(' docs/reviews/issue5-callbacks-actors/pinned/crates/agents/src/install.rs
```

New violations are `type.impl_trait.ret` and `expr.closure`.
Construct scoring charges respectively 5 and 3 tokens outside. Pricing
both as tolerated gives `3611 - (5 - 2) * 8 = 3587` for the file,
a 33-point saving (0.91%). The minimal edition 2024 form uses implicit
RPIT lifetime capture rather than unnecessarily adding an explicit `+ '_`
position. This is a distinct hypothetical rule, not a
side effect of 0046's accepted `pat.type` position.

These immediate immutable factory uses preserve named errors, explicit
branching, forward flow, value errors and repetition. A proposed rule would
still need to establish one idiom and confine the returned callable's use;
general stored/returned generators are not justified. The 16 consumers are
not automatically 16 unresolved friction points: the accepted local helpers
replace seven of them. **Nine inline mappings remain** in `put` (2),
`remove_others` (1), `remove` (1), `unpack` (2), `fetch` (1) and `drain` (2).
Those can satisfy the count test even though the score saving is small.
Comparing the accepted-local baseline with the same final factory gives
3613.5 to 3587.0 points, 3597 to 3562 tokens, and 483 to 478 lines.

The separate [error-factory assessment](issue5-error-factory.md) checks the
stable post-0046 consumer, its exact count, compilation and seven-quality
rule. It records nine unresolved sites, keeps all seven qualities with
fresh immediate factory uses, and provides focused compile/runtime and
ownership-failure evidence and a full real agents compile for
[proposal 0059](../proposals/0059-path-error-factory.md).
This review does not reject the factory merely for falling below the score
threshold and does not declare it accepted. The issue's original eleven-site
count belongs to its earlier source revision.

## Synchronous channel draining

`crates/agents/src/claude/session.rs:468`, `refusal`, builds a pinned async
`unfold` stream around `events.try_recv()`, filters it for the first
nonempty `TurnFinished.errors`, and awaits the first match. The only source
operation is synchronous. [from-fn.diff](issue5-callbacks-actors/from-fn.diff)
replaces that plumbing with an immediately consumed pull iterator.

The full function is 131.0 / 143 / 18 before and 108.5 / 117 / 13 after
when still outside-priced. `--by construct` charges exactly **two** closure
delimiter tokens outside, score 10. On admission the library `CallArg`'s
`Fixed::IfRestricted` mark applies to the tolerated construct, so boundary
repricing gives `108.5 - (5 - 1) * 2 = 100.5`: **23.3%** saving. The unrevised outside price
already gives **17.2%**, so the qualification does not depend on making the
new construct artificially cheap. There are no changed imports/callers.

This is one actual site. No `from_fn` or `spawn_blocking` exists in pinned
Conductor source. The count can be reproduced against the exact commit:

```sh
git -C /path/to/conductor grep -n -E 'try_recv|from_fn|spawn_blocking' \
  79568ce8a55b266fe1cf96b17dd5b2bc2dfa0c8c -- crates
```

`log_stderr` at `:452` genuinely awaits `next_line`, so its `unfold` stream
is retained. It is not another synchronous drain or another `from_fn` site.

Both before and after consume through the first nonempty error list, leave
later queued events untouched, use the same `errors.join("; ")`, and then
fall back to the same timed stderr join. `try_recv`'s empty and disconnected
errors both become `None` through the existing `.ok()`; no new error loss
is introduced. Neither drain waits for new messages. The old async closure
has no await, so neither version introduces a suspension while draining;
cancellation still occurs only around the subsequent real await.

The narrow candidate is `std::iter::from_fn` with an inline synchronous
zero-argument `receiver.try_recv().ok()` closure that pulls without blocking
from an external receiver, returns explicit `Option` termination, and is
the direct receiver of the eager terminal `find_map` in that expression.
Named callbacks, lazy-only chains, generator bindings/returns/helper
arguments, other terminal consumers, and blocking methods are excluded.
It admits neither retained generators nor new mutable
domain state. The proposed closure tier is tolerated and boundary use
applies to its direct library argument position; receiver methods keep their
existing treatment. Proposal 0058 records the seven-quality and
checker assessment. Owner decision remains pending.

## Sharing approval callbacks with current syntax

Original `crates/app/src/session.rs:216`, `:217` and `:221` differ only in
`Decision`. Each calls `on_decide((request, decision))` and focuses the card.
[ui-free.diff](issue5-callbacks-actors/ui-free.diff) extracts one ordinary
`decide_and_focus` function and keeps one inline callback wrapper per button.

The complete `TranscriptItem` plus helper grows from 673.0 / 799 / 124 to
677.5 / 805 / 132. Measuring the item alone would falsely suggest a
33-point saving; the helper costs 37.5. This is an existing repetition fix,
not score evidence for permitting escaping local callbacks.

It keeps all seven qualities: the same named decision values and effect
order, no new branching/mutation/error handling, a typed visible helper,
the existing inline UI wrappers and one shared body. A captured local
`let decide = ...` used inside the `onclick` closures remains excluded by
0046. The macro boundary does not make that capture an accepted construct.

## Blocking lock acquisition

Original installer `hold:190` and `attempt:201` poll `File::try_lock` every
200 ms. [blocking.diff](issue5-callbacks-actors/blocking.diff) removes polling,
passes an owned `File` into `spawn_blocking`, returns the locked file, and
retains it in `put` as `_lock` through the install. A closure that returned
only `()` would drop its lock-owning file too early; that is not measured.
Join failure becomes the existing typed I/O error through `Error::other`.

Complete affected `put + hold + attempt` scores are 438.0 before and 426.5
after at raw price, with `attempt` removed. Three closure tokens are outside;
their library-argument boundary repricing gives 414.5, saving **5.4%** for those functions.
Including the changed import and removed polling constant accounts for
the larger whole-file saving, 38.5 points (1.06%). There is only one lock
site, so neither ratchet test is satisfied.

Named state, branching, immutable file ownership, typed errors and shared
shape are retained by the inspected rewrite. Local reasoning and one idiom
need a rule about worker lifetime: cancellation no longer ends lock polling,
a started blocking worker cannot be aborted, and runtime shutdown may wait
for that worker. A contended lock also occupies a blocking thread instead
of sleeping an async task. These changes have not been tested. They cannot
be justified by the synchronous `from_fn` proposal.

## Actor initialization and multiple mutable values

Original session `Session::spawn:231` sends `Order::Start` into a newly made
16-slot inbox before exposing the handle. It discards a dummy oneshot reply.
The initial order is therefore first and its send cannot be full or
disconnected in this source. `SessionActor::run:432` has only the loop.

[actor-setup.diff](issue5-callbacks-actors/actor-setup.diff) passes the launch
to `run`, then invokes the existing complete `self.order(Order::Start(...))`
before the loop. It preserves the `offered` adjustment, its `ModelChanged`
event, `start` and `SessionReady`/`SessionClosed` events, and the existing
error propagation. `Order::Start` remains for later `Continue`, including
its idempotence branch. Calling `self.start(launch)` alone would bypass
model adjustment and is not an equivalent rewrite.

Startup remains inside the spawned session task. Later inbox commands are
still processed after startup, and no initial agent-event selection is lost:
the old task starts with `Agent::Down`. The initializer is performed even
if all public handles are dropped before its first poll; the old initial
queued order also remained to be handled before closed-inbox termination.

The complete affected `spawn + run` is 252.5 / 236 / 38 before and
258.5 / 230 / 38 raw after. Restore run's two receiver tokens from outside
to costly and its three loop tokens likewise: `258.5 - (5 - 3) * 5 = 248.5`,
only **1.6%** saving. For the whole file, loss of actor recognition also
makes eleven `&mut self` receiver definitions outside instead of their
existing boundary classification. Restoring those 33 tokens to boundary,
and the five run/loop tokens to costly, gives
`4626 - (5 - 1) * 33 - (5 - 3) * 5 = 4484`. This explicitly accounts for
the cascade instead of pricing all mutable receivers at one new tier.

Named launch/state, explicit startup order, confined actor mutation, typed
errors, local handler reuse and shared behavior are kept in this inspected
rewrite. One way would require one precise actor-setup form rather than
arbitrary statements before loops. One site and a 1.6% saving do not qualify.
The checker would need to preserve actor/owned-type discovery through a
defined initializer prefix; merely suppressing the loop violation would
leave the receiver cascade and mutation rules incorrect.

No exact multiple-`let mut` domain actor-state friction survived at the pin.
`digest` has two external mutable parameters (`reader`, `hash`), already
permitted by 0008; Codex `log_stderr` has one external `let mut lines` before
its drive loop, also permitted. Library mutable API values do not establish
a reason to split named domain loop state into multiple independent mutable
bindings. That would need real source and a separate seven-quality review.

## Validation and limits

Measurement and before/local/UI-helper subset checks passed. New candidate
checks fail only at the documented unsupported constructs (plus the actor
classification cascade); verbatim diagnostics are in the JSON.

For **both** the actual before and `from_fn` after agents crate, validation
passed `cargo check -p agents --all-targets`, all three focused channel
tests, and the two existing local CLI startup-refusal replay tests. The
same harness on both versions checks empty open/disconnected input,
non-refusal and empty-error events, the first nonempty error list, and
preservation of later events. Exact outputs are in
[validation-before.json](issue5-callbacks-actors/validation-before.json) and
[validation-after.json](issue5-callbacks-actors/validation-after.json).
Reproduce after materializing the pinned project with `measure.py --archive`:

```sh
python3 docs/reviews/issue5-callbacks-actors/validate.py --phase after --jobs 1
python3 docs/reviews/issue5-callbacks-actors/validate.py --phase before --jobs 1
```

Each uses an isolated project target and one build job. The after unit build
emitted one nonfatal incremental-cache permission note on the Windows
filesystem; every command exited 0. The existing consumer implementation
has its own gate for the allowed rewrites. No full daemon/UI build, browser
gate, Endpoint networking tests, or blocking-lock runtime pass is claimed
by this review. The full actual factory compile is recorded separately.

Not measured: arbitrary generators, retained callbacks, browser behavior
of the UI helper, other projects, the reconstructed original rounds, or
multiple mutable domain actor values. Blocking locks have not had their
behavioral extensions validated; returned-factory validation is recorded in
its separate assessment. No new construct is
declared accepted and no global checker/configuration has been changed.
