# Issue 5: returned path-error factories after accepted local helpers

The returned factory clears the **friction-count** test: **nine** inline
path-error mappings remain after accepted 0046. It does not clear the 15%
score test. A narrow rule can retain all seven qualities when each returned
callable goes immediately into one `Result::map_err`, and its concrete
constructor remains the single place that builds the named error.
[Proposal 0059](../proposals/0059-path-error-factory.md) is **proposed**, with
owner decision pending. No checker/config admission or consumer production
rewrite is included here.

## Actual source and the unresolved count

The published source is Conductor
`79568ce8a55b266fe1cf96b17dd5b2bc2dfa0c8c`, subset
`64352fdd5935de3954ee06b4598789cb89f67a03`, config version 14.
The baseline is that published `crates/agents/src/install.rs` **plus the
already accepted 0046 consumer changes**, not the pre-0046 file. The script
formats both identically and verifies that this reconstructed baseline is
identical to the coordinator's stable consumer installer. Baseline SHA-256:
`12d32fa7cac9e78ce7510d1ee5f09f0e60b674a81ca8f6022efdf0c4332ab78c`.

[baseline-local.diff](issue5-error-factory/baseline-local.diff) records the
accepted baseline changes; [factory.diff](issue5-error-factory/factory.diff)
is the complete additional installer rewrite. Pinned source plus these
diffs and [reproduce.py](issue5-error-factory/reproduce.py) reconstruct the
same complete before/after files inside this project's `target/`. This
avoids retaining another redundant copy of the installer.

Run the following on the reconstructed, post-0046 before file:

```sh
rg -n '\.map_err\(\|error\| io\(' \
  target/issue5-error-factory/before/crates/agents/src/install.rs
```

| Function | Remaining lines | Path context | Unresolved occurrences |
|---|---|---|---:|
| `put` | 153, 155 | `parent`, `&lock_path` | 2 |
| `remove_others` | 215 | `parent` | 1 |
| `remove` | 230 | `path` | 1 |
| `unpack` | 247, 252 | `binary`, `&staging` | 2 |
| `fetch` | 292 | `to` | 1 |
| `drain` | 307, 308 | `to`, `to` | 2 |
| Total | | | **9** |

Every row is a remaining immediate closure of the same kind: supplying the
path argument to the shared `io(path, error)` constructor while adapting a
library `std::io::Error`. They span six functions and cannot all be replaced
by one block-local helper under 0046. The two `drain` occurrences additionally
sit inside a whole-body 0007 loop; placing a helper before that loop is
currently excluded. These nine are unresolved sites, not nine invented new
callbacks. The historical issue's count of eleven is not reused.

The original sixteen adapter occurrences divide into those nine plus seven
already using three accepted local helpers: `download_error` three times,
`dir_error` twice, and `path_error` twice. Those seven are **not** counted as
unresolved friction. The complete candidate still rewrites them, because
the shared adapter shape should have one spelling across functions.

All direct constructor uses were audited with `rg -n '\bio\('`.
After 0046 the fourteen textual hits are nine inline mappings, three local
helper declarations, the direct `hold` construction at line 195, and the
constructor definition at line 344. The candidate leaves `hold` and the
constructor unchanged, removes the three local helpers, and introduces
one factory that forwards to that constructor. No second error-building
body or extra constructor is introduced.

## The complete rewrite and Rust behavior

```rust
fn io_error(path: &Path) -> impl FnOnce(std::io::Error) -> InstallError {
    move |error| io(path, error)
}
```

All sixteen adapters become `.map_err(io_error(path))`, with their actual
path expression. `hold` continues using `Err(io(path, error))` when its match
already has the error. `io` continues constructing the same owned path
String and retaining the same `std::io::Error` value.

Rust 2024 implicitly captures the path lifetime in the return-position
opaque type. The above signature compiles without an explicit `+ '_`;
there is no proposed lifetime-syntax extension. The closure captures only
the shared `&Path`; path rendering remains inside the closure and happens
only when `map_err` receives `Err`. Building the factory on an `Ok` does
not allocate the error's path String or construct an error.

The opaque return exposes **FnOnce, not Copy**. Although the underlying
closure can copy its shared reference, this does not permit the caller to
reuse the opaque callable. Replacing `let path_error = |error| ...` with
`let path_error = io_error(path)` and reusing it is an invalid rewrite.
The candidate removes those bindings and builds a fresh callable for each
adapter. A compile-negative example retains the observed E0382 for reuse.

The path owner remains borrowed through the direct `map_err` expression;
the returned `InstallError` is owned. Tests cover `&Path`, `&PathBuf`, a
temporary `PathBuf`, two consecutive uses, mutation of the original
`PathBuf` after mapping, and mapping after an await. Returning a callable
that borrows a local path is rejected with E0597. No factory callable may
be bound, stored, returned again, sent to a task, or passed through another
closure under the proposed rule, even where Rust alone would allow it.

## Measurements with all changed callers

[results.json](issue5-error-factory/results.json) records commands, complete
check diagnostics, whole-file totals, affected full-function rows,
outside-priced token counts, and compile/test output. The affected set is
the complete `put`, `remove_others`, `remove`, `unpack`, `fetch`, `drain`,
`sha256`, unchanged canonical `io`, and the new `io_error`. No import changes
occur. Including the existing constructor makes its retained cost explicit;
`hold` is unchanged and gains no saving.

| Region | Before | After raw outside price | After proposed price |
|---|---|---|---|
| All affected functions: score / tokens / tool lines | 1122.0 / 1121 / 136 | 1119.5 / 1086 / 131 | **1095.5 / 1086 / 131** |
| Complete installer: score / tokens / tool lines | 3613.5 / 3597 / 483 | 3611.0 / 3562 / 478 | **3587.0 / 3562 / 478** |

The proposed price is **tolerated, 2.0**, for the narrow `type.impl_trait.ret`
and factory's `expr.closure`. Boundary use **does not apply** to this
private factory definition: no external library requires its return shape.
The checker prices five return-type tokens plus three closure delimiter
tokens outside. Repricing those eight gives `raw - (5 - 2) * 8 = raw - 24`.
Existing call/body/boundary prices remain unchanged.

Affected score drops 26.5 points (**2.36%**), tokens drop 35 (**3.12%**),
and tool lines drop five. Whole-file score drops 26.5 (**0.73%**).
These are deliberately measured after the 0046 saving. Qualification rests
on **nine >= three** unresolved occurrences of one kind, not on the 15%
test or on counting the seven already resolved mappings twice.

## Validation, enforcement and limits

The before passes published v14 check. The after fails only at:

```text
crates/agents/src/install.rs:336:29: type.impl_trait.ret: `impl Trait` in a return type
crates/agents/src/install.rs:337:5: expr.closure: a closure outside a method call only as the argument of `Closure::new`, `unfold`, `ask` or a Dioxus hook, or an immutable local binding used only in calls in its block, directly or as an argument of the permitted closure-taking calls; never stored, aliased, unused or captured by another closure or async block (proposals 0006, 0017, 0018, 0026, 0046)
```

Compute-only `rustc --edition 2024 --test -D warnings` and
`clippy-driver --edition 2024 --test -D warnings` pass on the exact
factory/constructor and complete real synchronous `remove`/`remove_others`
functions. The harness mechanically keeps only the actual `InstallError::Io`
variant shape and omits its external derive/message attributes and the
consumer's tracing log statement. It does not stub filesystem APIs or
replace the returned callable with a different type. Six behavior and
borrow/call-shape tests pass; reuse and escape fail with the expected Rust
ownership diagnostics. Toolchain: `rustc 1.102.0-nightly (ac8310ba6 2026-11-09)`.

The **complete real agents crate**, all targets included, passed
`CARGO_BUILD_JOBS=1 cargo check -p agents --all-targets`, exit 0, in 1m13s.
The consumer agent performed the separately authorized reversible experiment
using that worktree's own cached target. It temporarily installed the exact
complete after file and restored the original installer in `finally`.
[full-agents-check.log](issue5-error-factory/full-agents-check.log),
[full-agents-result.json](issue5-error-factory/full-agents-result.json), and
the exact [guarded experiment script](issue5-error-factory/full-agents-experiment.py)
retain the compilation and restoration proof. Candidate SHA-256:
`5804caec05b8d2f30e2c2e25f7c0f984a2ac4f5e69861fbdf4799a5c539f63dc`.
Original and restored installer hashes both equal the baseline hash above;
original and restored **complete worktree diff** hashes both equal
`0b12a5eca06c18e47388fbd537af3d8a2357d7819996eb1101ce0c2e00eeb6bd`.
No other source/config or production behavior was changed by the experiment.
This is a full compile pass, plus the focused behavior/clippy checks above;
it is not a claim that full installer endpoint tests or the entire workspace
clippy/test gate ran for this proposal.

The checker change would recognize a private, nongeneric function with one
shared `&Path` context parameter, the concrete
`impl FnOnce(std::io::Error) -> OurErrorEnum` return, and a whole-body
immutable `move |error| constructor(path, error)`. Its uses are direct,
fresh arguments to `Result::map_err` only. All other return-position traits,
capturing contexts, callable storage/reuse and generator shapes remain out.
Function definition/use names must be linked; blindly allowing every
return `impl FnOnce` or every closure body would not implement this rule.
Rust name resolution and confirming that `map_err` is the immediate std
Result adapter remain review checks, as for current closure methods.

## Seven qualities

1. **Named states: kept.** The same `InstallError::Io` holds the owned path and typed cause; no bool or sentinel is introduced.
2. **Visible branching: kept.** The factory contains no branch; Result mapping and the existing match/`?` structure are unchanged.
3. **Forward data flow: kept.** Only an immutable shared path reference is captured; no mutable state or ownership mutation is added.
4. **Errors are values: kept.** The original I/O error moves unchanged through the canonical constructor; success remains success.
5. **Local reasoning: kept.** The private function's explicit concrete input, callable argument and named output state the dependency; immediate use bounds the borrow and prevents retained callback lifetimes.
6. **One way: kept.** Across-function path-error adaptation uses this one factory; the old per-call wrappers and local copies of that shared shape are replaced. Direct error construction still calls `io` when an error value is already in hand.
7. **Repetition is data: kept.** The variable path is the factory argument and the adapter shape occurs once; the concrete error construction itself is still written once.

Unmeasured: other repositories, richer captured contexts, multiple context
arguments, stored/reusable callable APIs, generic factories, async closures,
and new generator consumers. Full network installer endpoint tests are not
part of the factory validation; no download, lock, await or process behavior
changes in this rewrite.

Reproduce from the Subset root, using compute-only direct WSL execution if
its networking shell cannot start:

```sh
python3 docs/reviews/issue5-error-factory/reproduce.py \
  --conductor /path/to/read-only/conductor \
  --consumer /path/to/consumer-with-0046 \
  --subset /path/to/published-v14/subset
```
