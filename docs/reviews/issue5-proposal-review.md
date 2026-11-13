# Independent proposal review: 0058 synchronous receiver iterator

Decision after final validation follow-up: proposal-ready for owner consideration. Rule, measurement and advertised compiler/runtime validation gates are satisfied; no required correction or unreported validation blocker remains for the claims this proposal makes. This is a readiness review, never policy acceptance. Proposal 0058 remains `status: proposed`; the owner decides admission.

Reviewed the actual complete before/after `refusal` functions and retained diff at Conductor `79568ce8a55b266fe1cf96b17dd5b2bc2dfa0c8c`, the behavioral harness and validation script, raw scorer/measure/check records, version 14 configuration, published `docs/subset.md`, `.claude/skills/pair/SKILL.md`, `docs/presenting-a-case.md`, and relevant accepted proposals 0006, 0008, 0023, 0037 and 0046. Reviewed checker/scorer source to verify boundary repricing. Final execution evidence was reviewed at immutable Subset commit `d6c0bc87ed98aa878e7910c31b94517d45116021`, including both tracked validation JSON records and their exact native scratch log counterparts. No private implementer reasoning was used. Native Windows read tools only; no compilation, WSL execution, retry loop, code/proposal edit, or external message was performed. This review artifact is separate from the 0059 path-error-factory findings.

## Resolved corrections and completed validation

### 1. Resolved: explicit eager consumption and inline ownership confinement

The latest Rule now requires the `std::iter::from_fn` call to be the direct receiver of the eager `find_map` terminal, using the already permitted inline adapter closure. Only the terminal result may be bound or returned. It explicitly excludes generator bindings, returns, aliases, helper arguments and captures, named callbacks, lazy-only adapters, and other unmeasured terminals. This resolves the original ambiguity caused by describing consumption only through the general adapter list.

The revised normative rule excludes these examples:

```rust
let pending = std::iter::from_fn(|| events.try_recv().ok()).map(identity);
return std::iter::from_fn(|| events.try_recv().ok()).take(4);
consume(std::iter::from_fn(|| events.try_recv().ok()).map(identity));
```

The Tool change also explicitly excludes `let pull = || events.try_recv().ok(); std::iter::from_fn(pull).find_map(...)` and warns against adding `from_fn` to the shared final-segment callee whitelist, which would accidentally admit named callbacks through 0046. Its exact qualified call/body/consumer shape is precise enough to implement with the existing syntax-based checker. Negative fixtures for lazy escape, callback capture and named callbacks remain appropriate implementation checks if admission is later authorized; they are not a prerequisite checker change before the owner decides.

### 2. Resolved: exact nonblocking pull and documented semantic limits

The latest Rule requires available values to be pulled without blocking and restricts the closure body to the existing `receiver.try_recv().ok()` chain. It explicitly excludes `blocking_recv`, arbitrary other receiver methods and general generators. This resolves the original overbreadth: merely saying synchronous external `&mut` receiver would also have described Tokio's blocking receive, which waits and can panic inside an async runtime.

The exact existing pull form preserves the measured Empty/Disconnected-to-None error contract. Future receivers still need review of their real error behavior: a similarly spelled API with additional failures is not justified merely by matching `.try_recv().ok()`. Existing error rules remain in force, and the rule no longer admits arbitrary call-chain-to-Option bodies.

The Tool change now expressly leaves type origin, nonblocking pull/Option semantics and ownership to review plus compilation; it does not claim name-based syntax proves library behavior. Compilation establishes Rust types and ownership, while review establishes pull/error semantics. Combined with the specific `find_map` consumer and same-expression lifetime, this is sufficiently precise for implementation without claiming unprovable type facts.

### 3. Resolved: advertised before/after validation completed

The earlier review correctly withheld readiness while completion records were absent. At final follow-up, both `validation-before.json` and `validation-after.json` are tracked at `d6c0bc8` and contain three successful commands each. Both identify the pinned Conductor commit and one build job. I verified that all six embedded command outputs exactly match the respective native scratch log contents after newline normalization.

| Command | Before | After |
|---|---|---|
| `cargo check -p agents --all-targets` | Exit 0, completion in 2m04s | Exit 0, completion in 3m29s |
| `cargo test -p agents --lib issue5_from_fn_review_tests` | Exit 0, 3 passed, 0 failed | Exit 0, 3 passed, 0 failed |
| `cargo test -p agents --test replay fails_with` | Exit 0, 2 passed, 0 failed | Exit 0, 2 passed, 0 failed |

Both harness outputs name the same three inspected channel tests, with 44 unrelated tests filtered. Both replay outputs name `a_cli_that_exits_at_start_fails_with_its_last_stderr_line` and `a_resume_with_no_transcript_fails_with_the_clis_reason`, with 11 unrelated tests filtered. These are actual nonzero test runs, not merely successful compilation of tests. The script resets actual complete source files from the appropriate before/from-fn snapshot, appends the same harness and confines builds to the isolated project target. No additional candidate behavior is mixed into this phase.

The after unit build contains one nonfatal incremental-cache permission note on the Windows filesystem; its text concerns reuse of compilation work, and compilation plus tests still finish with exit 0. The proposal reports this note accurately. Full Endpoint/networking tests and the broader Conductor Clippy/test gate are expressly not claimed. Those limitations do not undermine the full-agents compile and focused before/after validation claims. I did not rerun any build or test.

## Boundary correction identified and incorporated during review

The initial draft incorrectly denied boundary use and computed `108.5 - (5 - 2) * 2 = 102.5`. That rationale copied 0046's local-declaration treatment to a direct library argument. The latest proposal read during this review has corrected boundary use and the numbers to 100.5 / 4175.5. This correction is resolved in that version and must remain consistent across the associated report.

The current implementation establishes the result without executing a build:

- `walk::visit_expr` assigns `Slot::CallArg` to the `std::iter::from_fn` closure and marks library argument expressions `Fixed::IfRestricted`.
- `checks::library_value` covers a `CallArg` whose callee is `Owner::Theirs`.
- `config::Tier::restricted` includes `Tolerated`.
- `score::judge` charges violations outside before considering boundary marks. Once admitted, the same marked tolerated closure is charged boundary 1.0.

Thus the raw outside row cannot establish the admitted tier by itself. Its existing mark is masked by the violation. Accepted `unfold` closures already have boundary rows in the raw before evidence; a local declaration in 0046 has no corresponding library argument mark.

## Behavior, ownership and cancellation assessment

For the actual rewritten `refusal`, static source review supports equivalence:

| Input/state | Before and after behavior |
|---|---|
| Empty channel with a sender still alive | `try_recv` reports Empty; `.ok()` produces `None`; drain ends and stderr fallback is awaited. Neither waits for another sender message. |
| Disconnected and empty | Disconnected also becomes `None`, taking the same fallback. |
| Disconnected with buffered messages | Buffered events are still delivered by `try_recv` before Disconnected; the same event selection applies. |
| Other event variants | Event is consumed and skipped by the same wildcard match arm. |
| `TurnFinished` with an empty `errors` list | Event is consumed; `first()` returns `None`; selection continues. |
| First `TurnFinished` with a nonempty error list | The list is joined with the identical `"; "` separator and returned. A list containing an empty string still qualifies in both forms; nonempty refers to list cardinality. |
| Events after that first qualifying event | Neither `filter_map(...).next()` nor `find_map` requests another source item after finding `Some`; later events stay queued. |
| No qualifying event before a no-value poll | Same timed `stderr_reason` call and fallback string behavior. |

Both versions repeatedly observe the receiver while draining; neither establishes an atomic snapshot against concurrent producers. The rewrite preserves this polling model rather than promising a fixed queue cutoff.

The old async closure has no await and yields a ready result when polled. Its source/filter/next path has no source suspension; `.await` syntax itself does not guarantee that `poll` returns Pending. The new synchronous `find_map` similarly runs in the enclosing future's poll. Cancellation before first poll consumes nothing in either version; there is no new scheduling/cancellation opportunity between drained events. A genuine subsequent stderr suspension remains in the same branch. Real `log_stderr` awaits `next_line` and correctly stays an async stream. The proposal does not justify blocking work or a retained generator.

The old unfold moves the existing mutable receiver reference through its library state and ready closure futures. The new inline closure captures/reborrows that same reference for immediate `find_map`; its temporary iterator is dropped when the expression completes. Returned strings own their contents, so no receiver borrow escapes through `errors`. Rust compilation must still verify inferred capture/lifetimes in the full crate. No new lifetime annotation, mutable domain binding, counter or closure signature is needed.

The complete source suffix starting at `stderr_reason` is identical between retained before and after. Timeout, JoinError and absent stderr are still flattened through the same `.ok().and_then(Result::ok).flatten()` chain and return `"it printed no reason"`. Selecting channel errors still drops the unused `JoinHandle`; cancelling or timing out fallback still drops its handle. As before, dropping a Tokio JoinHandle detaches rather than aborting its spawned task. No new cancellation ownership contract is introduced by the iterator.

## Seven qualities

For the measured function, all seven are kept relative to the accepted subset, with 0008's existing confined external mutation compromise retained:

1. Named states: identical `ClaudeEvent`, `Option` termination and owned refusal string; no new flags/sentinels.
2. Visible branching: identical event match and stderr fallback match. `.ok()` is existing behavior, not new hidden error branching.
3. Forward data flow: mutation remains confined to the external receiver API under 0008. Inline capture does not add domain mutation or permit retained mutable state.
4. Errors are values: unchanged Empty/Disconnected conversion, error list selection and stderr failure handling. The revised rule excludes arbitrary new pull/error conversions.
5. Local reasoning: source call, termination and eager consumer are adjacent. The revised rule explicitly excludes lazy escape and blocking pulls.
6. One way: this exact nonblocking receiver drain has one form; genuinely awaited sources keep streams. Other terminals and receiver methods require independent evidence.
7. Repetition is data: one selection body processes every queued event; no per-event duplication is introduced.

No prior rejection of this specific nonblocking, immediately consumed form was found in the consulted decisions. 0046 explicitly leaves `from_fn` to a separate case; this measured case is independent of accepted local-closure friction.

## Raw measurement assessment

The retained raw records agree with the complete real function, full-file scope and single diff:

| Scope | Before score / tokens / lines | After raw | After at tolerated tier with existing boundary use |
|---|---|---|---|
| Complete `refusal` | 131.0 / 143 / 18 | 108.5 / 117 / 13 | 100.5 / 117 / 13 |
| Complete Claude session file | 4206.0 / 4363 / 715 | 4183.5 / 4337 / 710 | 4175.5 / 4337 / 710 |

The after construct rows contain one outside `expr.closure`, count 1, tokens 2, score 10.0, plus a separate unaffected boundary `expr.closure`, count 1, tokens 2, score 2.0. No other after row is outside. Reprice only the two violating tokens: `108.5 - (5 - 1) * 2 = 100.5`; full file `4183.5 - 8 = 4175.5`. Do not reprice every occurrence of `expr.closure` or change already boundary tokens.

The measured affected saving is 30.5 / 131 = 23.3%; token saving 26 / 143 = 18.2%; line saving 5 / 18 = 27.8%. Even outside-priced, the saving is 22.5 / 131 = 17.2%, so the score threshold is met independently of the disputed boundary rationale. The whole-file saving is 30.5 / 4206 = 0.73%, reported separately rather than substituted for affected-code qualification.

The diff changes only `refusal`: no imports, caller changes, helpers or removed function costs are omitted. `measure.py` formats both versions consistently and records `--by file`, `--by fn`, `--by construct`, `measure`, and `check` commands and exact outputs/exits. Raw before `check` exits 0; raw after exits 1 with precisely the documented unsupported closure diagnostic. This failure is appropriate for a proposed construct and must not be suppressed before admission. One friction site is supported; qualification uses the score test rather than claiming three sites.

## Harness and implementation limits

The three supplied named tests meaningfully cover empty open/no-wait fallback, disconnected empty fallback, and a mixed sequence of other event/empty list/first multi-error list/later event with queue inspection. The three-test set does not trigger the pair skill's four-case table threshold. `validate.py` resets actual source files independently for each phase, appends the same harness, checks all agents targets, runs the named harness and filters replay tests by `fails_with`, which matches the two inspected startup-refusal cases.

The harness does not directly exercise buffered-disconnected input, all-skipped fallback, fallback timeout/JoinError/None, cancellation, or concurrent producers. The unchanged code and source reasoning above support those assessments; no runtime coverage is claimed for them. The final retained output establishes that the promised three harness tests and two startup replay tests ran successfully on both actual versions. Type/name resolution, receiver behavior and callback retention cannot be proven by a syntax-only checker; the proposal correctly leaves their real facts to review and compilation under the explicit nonblocking/termination limits.

The final follow-up reread the proposal, associated report and completed validation evidence at `d6c0bc8`. Both normative clarifications and boundary repricing remain resolved, and all advertised before/after compiler/channel/replay checks now pass. No additional rule, measurement or readiness correction is required. The proposal is ready for owner consideration and remains proposed; that assessment grants no permission to alter `subset.toml`, publish a new rule, or apply the production rewrite. Commit/push of the completed coordinated work remains the root coordinator's responsibility under the workspace instructions.
