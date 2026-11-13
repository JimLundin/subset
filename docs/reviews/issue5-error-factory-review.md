# Independent proposal review: 0059 path-error factory

Decision after follow-up: proposal-ready for owner consideration. The context-expression correction is resolved, and the measured real rewrite, qualification, pricing and recorded validation are supported. No required correction or unreported validation blocker remains for the claims this proposal makes. Proposal 0059 remains `status: proposed`; reviewer readiness is never owner acceptance.

Reviewed immutable commit `37843b656eeb2312f4696a5964ee41ee264c85a6` in `issue5-patterns` and its doc-only corrective successor `878e676de05b5618d9a12b07c0c1099fc71ca061`: proposal 0059, its public review, complete baseline/candidate diffs, reconstructed full installer snapshots, measurement/check output, reduced harness and test/ownership output, full-agents compilation log, restoration record and guarded experiment script. Baseline source is Conductor `79568ce8a55b266fe1cf96b17dd5b2bc2dfa0c8c` plus accepted 0046 installer rewrites, with Subset `64352fdd5935de3954ee06b4598789cb89f67a03`, config v14. Consulted published subset rules, the pair skill, presenting-a-case requirements and prior 0046/0026 decisions. No private author reasoning was read. Native Windows read/git tools only; no new builds, WSL execution, source/proposal edits, agents or external messages. This is distinct from the 0058 review.

## Resolved correction: restrict eager path-context expressions

The factory body is correctly effect-free until called: it creates a closure capturing one shared reference and does not invoke `io`. However, `.map_err(factory(context))` evaluates `context` to create that closure even when the receiver Result is `Ok`. In the original inline form, a context expression in `|error| io(context, error)` is evaluated only on `Err`. The original draft restricted the factory definition and immediate consumer without restricting its context argument syntax.

For example, that original wording could be read as permitting:

```rust
// Before: make_path runs only on Err.
operation().map_err(|error| io(&make_path(), error))

// After: make_path also runs on Ok.
operation().map_err(io_error(&make_path()))
```

Even a helper returning a shared `&Path` can perform an external side effect, allocate or panic before returning it. Signature immutability and immediate `map_err` consumption do not prove pure context evaluation. Allowing arbitrary expressions therefore exceeds the source evidence and can weaken forward flow, error behavior and local reasoning.

The corrected Rule and Tool change at `878e676` now require a plain existing `&Path` binding or shared `&name` of an existing immutable `PathBuf` binding, explicitly excluding calls, methods, fields, indexing, temporary construction, dereferencing and other computed expressions. This is precise enough to exclude the eager-effect counterexample and stays within all real measured sites. Actual `Path`/`PathBuf` origins and coercion behavior still need review plus Rust compilation; a same-spelled custom type or custom coercion cannot be proven pure by syntax alone. Other context-expression forms require independent evidence.

The real sixteen candidate arguments are exactly stable `parent`, `path`, `download`, `dir`, `binary`, `to`, `&lock_path` and `&staging` forms, so this correction does not require changing or remeasuring the installer. Both corrected documents explicitly distinguish the harness's temporary `PathBuf` ownership/lifetime proof from admitted subset syntax. The follow-up commit changes only these two documents; it leaves source, tests, measurements and compile/restoration records unchanged.

## Qualification after 0046

The count correctly starts after accepted local-helper changes, not at the historical sixteen inline sites:

| Function | Remaining inline sites | Context |
|---|---:|---|
| `put` | 2 | `parent`, `&lock_path` |
| `remove_others` | 1 | `parent` |
| `remove` | 1 | `path` |
| `unpack` | 2 | `binary`, `&staging` |
| `fetch` | 1 | `to` |
| `drain` | 2 | `to`, `to` |
| Total | **9** | Six complete caller functions |

The raw line/source records agree with the reconstructed baseline and `rg` patterns. The seven other original mappings already use `download_error` three times, `dir_error` twice and `path_error` twice; those seven resolved occurrences are explicitly not counted again. The remaining nine still-inline adapters have one shared shape: adding path context to the same concrete `std::io::Error` through `io(path, error)` across functions. One block-local helper cannot serve those six functions under 0046. The two drain occurrences additionally sit inside the existing whole-body loop, where a pre-loop helper declaration is excluded.

Nine occurrences satisfy the OR friction-count threshold of at least three of one kind. The score saving is 2.36%, so the proposal correctly does not claim the separate 15% score test. 0046 explicitly left returned factories to an independent case; this is the narrower post-0046 unresolved evidence that the earlier callbacks review lacked.

## Complete rewrite and behavior

The complete installer diff changes all sixteen `map_err` consumers and removes all three resolved local closure declarations for one consistent spelling. It adds one factory forwarding to the existing `io` constructor. The actual `io` body is byte-identical between reconstructed before and after. `hold` is also byte-identical and retains its direct `Err(io(path, error))` arm, so error construction is not duplicated to inflate savings.

All filesystem operations, `or_else`, archive branches, lock loop, waits, process calls, downloads, hashing and matches are unchanged. Each `Err` invokes the same constructor with the same stable path and original owned I/O error. `path.display().to_string()` remains inside that constructor, which is invoked only if `map_err` receives Err; producing the adapter on Ok does not render the path or allocate an error string. Success drops a closure holding only a shared reference, with no new destructor work. Existing await expressions stay before the immediate factory/mapping expression; no extra suspension or retained task is introduced.

The proposal correctly handles opaque-callable ownership. Rust 2024 captures the sole path lifetime in the return-position opaque type without `+ '_`. The exposed bound is `FnOnce(std::io::Error) -> InstallError`, not Copy or a reusable `Fn` contract. Although the hidden closure's shared capture could be Copy, callers do not acquire that bound from the opaque return. Replacing a reusable local helper with one bound factory result would move it at the first `map_err` and fail on reuse. The candidate instead creates a fresh result at all sixteen consumers. Recorded E0382 rejects reuse; recorded E0597 rejects a result escaping a borrowed local path. Immediate successful/failed mapping releases the shared borrow, while the produced error owns its path string and I/O cause.

## Measurements and pricing

The complete affected set includes seven changed caller functions (`put`, `remove_others`, `remove`, `unpack`, `fetch`, `drain`, `sha256`), unchanged canonical `io`, and new `io_error`. Local-helper removal is included inside the full caller rows. `hold` is unchanged and gains no saving. No import or other call site changes are omitted.

| Scope | Before score / tokens / lines | After raw | After proposed tolerated price |
|---|---|---|---|
| Complete affected functions | 1122.0 / 1121 / 136 | 1119.5 / 1086 / 131 | 1095.5 / 1086 / 131 |
| Complete installer | 3613.5 / 3597 / 483 | 3611.0 / 3562 / 478 | 3587.0 / 3562 / 478 |

I independently summed the retained full-function rows: before 1122/1121/136, after 1119.5/1086/131. The affected and whole-file raw deltas are both -2.5 points and -35 tokens; adding the new factory is fully charged, rather than presenting caller-only savings. The complete canonical constructor retains its 37-point / 38-token cost on both sides.

After outside rows are exactly `type.impl_trait.ret`: count 1, five tokens, score 25; and `expr.closure`: count 1, three tokens, score 15. No other outside row exists. Repricing only those eight tokens at tolerated 2.0 gives `raw - (5 - 2) * 8 = raw - 24`, yielding the published totals. Affected saving is 26.5/1122 = 2.36%; tokens 35/1121 = 3.12%; whole-file saving 26.5/3613.5 = 0.73%.

No boundary correction is needed for this proposal. The returned closure is the tail of our private function, not a direct library call argument; the opaque return is our function's chosen signature. Neither has a library argument or library-required signature boundary mark. Existing boundary totals remain 161.0 before and after. Passing the completed factory call into library `map_err` does not make the factory's separate definition a boundary declaration. This differs from 0058's inline library constructor argument.

Before v14 `check` exits 0 with empty stdout/stderr. After exits 1 with exactly the two documented return-impl-trait and returned-closure diagnostics. That failure is expected for a still-proposed construct; there is no broad admission/configuration change in the reviewed commit. The measured Conductor config is v14 and its recorded weights support the stated arithmetic.

## Recorded validation and restoration

The recorded reduced `rustc --edition 2024 --test -D warnings` and `clippy-driver --edition 2024 --test -D warnings` commands both exit 0 with empty stderr. The behavior command runs **six** tests, with six passed and zero failures/filtered tests. The reconstructed harness contains the actual `io` and `io_error` bodies plus complete synchronous `remove` and `remove_others`; only tracing and external error-derive/message attributes are removed. It does not substitute filesystem or Result APIs.

The six tests exercise owned error path/kind/text, two consecutive fresh uses against reusable local before helpers, PathBuf and temporary path lifetimes, success/direct-constructor compatibility, actual keep/lock/removal behavior, real read-directory failure propagation and immediate post-await mapping. The success test does not instrument allocation or constructor invocation counts; the exact source body establishes that the constructor remains conditional. The temporary PathBuf test demonstrates Rust lifetime support rather than authorizing general eager context expressions. Rust ownership negative output contains the claimed E0382 and E0597. The captured toolchain is `rustc 1.102.0-nightly (ac8310ba6 2026-11-09)`.

The full-agents log contains the actual agents/proto paths and ends in a successful dev-profile completion in **1m13s**. The guarded script's concrete command is `cargo check -p agents --all-targets` with `CARGO_BUILD_JOBS=1`; its record has `cargo_exit_code: 0`. It uses the consumer's own target cache, validates the original installer hash before mutation, writes only that candidate source file, manages only its own cargo process group, and restores the original bytes in `finally`.

I independently hashed the native reconstructed snapshots and verified:

- Before SHA-256: `12d32fa7cac9e78ce7510d1ee5f09f0e60b674a81ca8f6022efdf0c4332ab78c`, matching the experiment's original and restored hashes.
- After SHA-256: `5804caec05b8d2f30e2c2e25f7c0f984a2ac4f5e69861fbdf4799a5c539f63dc`, matching the exact compiled candidate hash.
- The record's original and restored `git diff --binary HEAD` hashes both equal `0b12a5eca06c18e47388fbd537af3d8a2357d7819996eb1101ce0c2e00eeb6bd`, and both restoration flags are true.

Those diff hashes prove restoration of the tracked worktree diff recorded by that command. They do not prove all untracked files were restored; the experiment intentionally creates `.audit` backup/log/record artifacts. The source-restoration claim is supported and no restoration blocker is identified. The proposal accurately distinguishes full-crate compilation and reduced behavior/Clippy results from unrun full endpoint tests or workspace-wide Clippy/tests. I did not rerun any of these commands. Commit whitespace checking also passed.

## Scope and checker assessment

The definition rule is otherwise narrow enough: one private nongeneric free function, one shared immutable Path argument, concrete std I/O error and owned named error output, one `move |error| constructor(path, error)` tail, and no added captures, traits, lifetimes, mutable context or async work. Existing subset constraints continue to apply inside the canonical constructor.

Enforcement must link each recognized factory definition to every permitted immediate `.map_err(factory(path))` consumer and reject other value uses, aliases, captures, returned-again values and storage. Blindly adding return `impl Trait`, permitting all `FnOnce` returns, or permitting arbitrary returned closures would not implement this proposal. With the corrected context-expression restriction, syntax can conservatively check the exact contract and use positions. Actual `std::path::Path`, std `Result::map_err`, constructor/type identity, and coercion semantics still require review and compilation; same final names do not prove their behavior. No new checker implementation is authorized by review alone.

## Seven qualities

For the measured installer rewrite, all seven are kept:

1. Named states: same `InstallError::Io` with owned path and typed I/O cause.
2. Visible branching: same Result adapters, matches, `or_else` and `?`; the factory has no branch.
3. Forward data flow: only one shared immutable path reference is captured; no new mutable domain value or accumulator.
4. Errors are values: the original I/O error moves through the same canonical constructor only on Err, with no additional failure handling.
5. Local reasoning: concrete callable input/output types identify the dependency, and immediate use bounds its lifetime.
6. One way: one factory supplies the shape shared across functions; direct concrete construction remains for already-bound errors. Other local helpers retain 0046.
7. Repetition is data: paths vary as arguments while both adaptation and canonical construction each have one body.

For the generalized proposal, the corrected context-expression rule preserves qualities 3/4/5 by excluding eager computed contexts. No other readiness correction is identified. The recorded compile and focused behavior evidence are sufficient for the claims the proposal makes; they do not declare unrun project gates passed. The final reviewed proposal is ready for owner consideration and remains proposed. Commit/push and presentation to the owner remain the root coordinator's responsibility.
