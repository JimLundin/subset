# Conductor issue #5 consumer review

Reviewer: independent GPT-6.1 Sol.
Worktree: `/home/jim/projects/apps/conductor-issue5-v14`.
Base: `79568ce8a55b266fe1cf96b17dd5b2bc2dfa0c8c`.
Reviewed: stable uncommitted diff, `.audit/subset-issue5-v14.friction.md`, Conductor `AGENTS.md`, `.claude/skills/pair/SKILL.md`, published `docs/subset.md`, and consumer `subset.toml` (version 14). No implementer reasoning was provided or consulted.

## Required fixes

None found in the Rust changes or friction log. The manual review passes correctness, volume, and subset-quality review. Final acceptance still requires the coordinator's verification of the complete `tools/check.sh` gate.

## Correctness assessment

- `crates/daemon/src/log.rs:107`: the grouped `kind @ (...)` pattern retains ownership of the entire original event kind, including its text. Both delta variants still send exactly one live event with the current head sequence; neither enters the stored log. Other events still increment the head, enter replay, and go live. The added regression at line 185 exercises assistant and reasoning deltas between stored events, checks live order and sequence values, and checks replay exclusion. No new event-drop or actor race path is introduced.
- `crates/agents/src/install.rs:241` and `:315`: confined local closures capture shared `&Path` references, which are copyable. Their use by value in repeated `map_err` calls preserves the original path for each error. No closure escapes or is captured by another closure or async block. Creation does not perform I/O, format an error, acquire a lock, or change the cancellation point. Each underlying `std::io::Error` still reaches `InstallError::Io` intact.
- `crates/agents/src/bin/s1.rs:409`: the local `chosen` helper captures the shared CLI path and constructs the same owned options at both spawn calls. Resume and log-path values are moved into their options as before; no borrowed option data escapes. The original helper was private and used only by this function.
- `crates/app/src/session.rs:272`: the shared helper calls `on_decide((request, decision))` before `document::eval(&focus_on(request))`, preserving both execution order and the original focus target. The event handler is invoked at click time. `RequestId` and `Decision` are `Copy`, so passing them into the helper does not consume state needed by the callback or focus script.
- `crates/daemon/src/registry.rs:245` and `crates/daemon/src/session.rs:77`: shared-reference destructuring copies only the `Copy` enums `Provider` and `Effort`. Registry handles remain borrowed until cloned on the successful branch; readiness failure still becomes the same provider-unavailable rejection. The predicate still compares the same effort values and returns the same offered item.
- `crates/proto/src/frame.rs:24` and `:38`: `MAX_FRAME_LEN` is a `u32` constant of 16,777,216. The inclusive unsigned ranges accept zero through the exact maximum, reject maximum plus one through `u32::MAX`, and retain the failed `usize`-to-`u32` conversion path in encoding. `body_len` retains its checked `usize` conversion, including the error behavior on a smaller target. Encoding retains the original `u64::MAX` fallback when reporting a length not representable as `u64`. Existing header tests exercise the exact limit and limit plus one.
- Array rewrites preserve owned iteration under Rust edition 2024. The removed `Vec::from` wrappers performed no filtering or transformation. Array argument evaluation and awaited reads remain left to right; concatenation and chains preserve the same event order and contents. Multi-field test rows remain named structs.

## Volume and subset qualities

The helper extractions consolidate existing repeated shapes without adding options, variants, layers, or new public APIs. Array receivers remove temporary vector allocation and wrapping where the values are consumed immediately. Fixed test-case arrays remain immutable. The remaining friction entries describe actual unchanged code sites and do not claim authorization for new syntax.

| Quality | Assessment |
|---|---|
| Named states | Existing enum states remain intact; grouped patterns preserve complete variants. No new flags or sentinels. |
| Visible branching | Decisions remain exhaustive matches or existing permitted predicates; ranges express the old constant bounds directly. |
| Forward data flow | New locals are immutable and capture shared references. Existing loop state and library effects retain their scope. |
| Errors are values | Existing `Result` propagation, error context, and causes are preserved. |
| Local reasoning | Local helpers stay inside their owning function; the callback helper states and performs its complete two-step action. |
| One way | The rewrites use published version-14 forms and the established error, predicate, and callback idioms. |
| Repetition is data | Common shapes are shared once; test tables retain their named rows and single comparison. |

## Verification limits and native diff observation

Review used read-only native Windows git and UNC source reads. No WSL launch, cargo invocation, compilation, or test execution was performed because the shared host was under severe memory pressure. The implementer and coordinator own the complete gate and final checks.

Native Windows git additionally displayed `CLAUDE.md` as a deleted mode-120000 symlink to `AGENTS.md`. UNC `Get-Item` still sees `CLAUDE.md` as a reparse point. This is consistent with a Windows/UNC symlink observation artifact; the coordinator should confirm actual Linux git status before committing. It is not treated as a confirmed code deletion or a required Rust fix.
