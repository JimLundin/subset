---
construct: type.impl_trait.ret, expr.closure (private path-error factory)
status: proposed
task: Conductor issue #5, returned error adapters after accepted 0046
---

# Build one immediate path-error adapter across functions

## Rule

A private nongeneric free function may take one immutable shared `&Path`
parameter and return `impl FnOnce(std::io::Error) -> OurErrorEnum`. Its
whole body is an immutable `move |error| constructor(path, error)` forwarding
to one existing private concrete error constructor. No additional captures,
mutable parameters, generic bounds, explicit lifetime extensions, async
body, returned trait kind, or error payload shape is admitted.

Every callable is created fresh as the direct argument of
`Result::map_err`: `.map_err(factory(path))`. It is never bound, stored,
aliased, reused, returned further, captured, spawned, or passed through
another helper. The across-function path-error adapter shape uses this
one factory rather than repeated inline/local wrappers; 0046 still handles
other helpers confined to one block. When an error value is already bound,
its direct constructor call remains the form, including `hold`'s match arm.

## Before

Conductor at `79568ce8a55b266fe1cf96b17dd5b2bc2dfa0c8c`, plus the accepted
0046 installer local-helper rewrites, with subset revision
`64352fdd5935de3954ee06b4598789cb89f67a03` and config version 14:

```rust
std::fs::create_dir_all(parent).map_err(|error| io(parent, error))?;
let lock_path = parent.join("lock");
let lock = File::create(&lock_path).map_err(|error| io(&lock_path, error))?;
```

The existing `unpack` and `sha256` mappings already use three reusable
local closures. Nine inline occurrences remain elsewhere. The complete
same-file baseline and rewrite are reproducible from the pinned source,
[baseline-local.diff](../reviews/issue5-error-factory/baseline-local.diff),
and [factory.diff](../reviews/issue5-error-factory/factory.diff), not from
these illustrative two lines alone.

## After

```rust
fn io_error(path: &Path) -> impl FnOnce(std::io::Error) -> InstallError {
    move |error| io(path, error)
}

// In put:
std::fs::create_dir_all(parent).map_err(io_error(parent))?;
let lock_path = parent.join("lock");
let lock = File::create(&lock_path).map_err(io_error(&lock_path))?;
```

The complete rewrite replaces all sixteen adapter consumers, removing the
three local helper declarations. It preserves the existing `io` constructor
and its direct call in `hold`. That keeps one actual error-construction body.
The opaque `FnOnce` does not expose Copy: the three old local closures cannot
be replaced with a bound factory result and reused. Each adapter gets a
fresh factory call instead. Rust 2024 captures the path lifetime implicitly;
the minimal signature compiles without `+ '_`.

## Weight

**Tolerated, 2.0**, for the narrowly recognized return type and returned
closure. A named concrete callable contract and immediate-use restriction
keep its implicit shared context bounded, at the same tier as a local
closure. Boundary use **does not apply** to its private definition; no
library requires the factory's return shape. Ordinary body/call/boundary
weights remain unchanged.

## Measurements

| | Affected score | Tokens | Tool lines |
|---|---:|---:|---:|
| Before, after accepted 0046 | 1122.0 | 1121 | 136 |
| After, raw outside price | 1119.5 | 1086 | 131 |
| After, proposed tolerated price | **1095.5** | **1086** | **131** |
| Change at proposed price | **-26.5 (-2.36%)** | **-35 (-3.12%)** | **-5** |

The affected set includes the seven complete changed caller functions, the
retained canonical `io`, and new `io_error`. Whole-file score/tokens/lines
are 3613.5/3597/483 before, 3611.0/3562/478 raw after, and
**3587.0/3562/478** at the proposed price, a **0.73%** whole-file saving.
No imports change. Repricing five return-type tokens and three closure
delimiter tokens uses `raw - (5 - 2) * 8 = raw - 24`.

**Friction points removed: nine** still-inline path-error adapters across
six functions after 0046. The seven mappings already resolved by its three
local helpers are rewritten for consistency but **not counted again**.
This meets the **at least three of one kind** test; it does not meet the
15% score test. Exact site lines, checks, selected full-function metrics
and commands are in [the review](../reviews/issue5-error-factory.md) and
[results.json](../reviews/issue5-error-factory/results.json).

## Qualities

1. Named states: **kept**. The same named error variant owns path context and the typed cause.
2. Visible branching: **kept**. The factory has no branch; Result mapping and existing matches remain explicit.
3. Forward data flow: **kept**. Captures are shared and immutable; no mutable domain state is added.
4. Errors are values: **kept**. The same I/O error moves through the one canonical constructor, only on Err.
5. Local reasoning: **kept**. The explicit concrete callable contract and immediate consumption show and bound its path dependency.
6. One way: **kept**. The shared across-function adapter has one factory form; direct construction stays a concrete function call when the error is already available.
7. Repetition is data: **kept**. The varying path is an argument; neither adapter shape nor construction is copied across callers.

## Validation and tool change

Compute-only rustc and clippy-driver checks pass on the actual constructor,
factory and complete real synchronous consumers, with external logging and
derive attributes omitted in the reduced harness. Six tests cover owned
error contents, fresh consecutive calls, path/temporary borrows, immediate
post-await mapping and real filesystem consumers. Rust rejects reusing the
opaque callable with E0382 and escaping a borrowed local path with E0597.
The complete real agents crate also passed
`CARGO_BUILD_JOBS=1 cargo check -p agents --all-targets`, exit 0. The
separately authorized experiment installed the exact complete candidate file
only temporarily and restored the original installer bytes and full
worktree diff. [The full compile output](../reviews/issue5-error-factory/full-agents-check.log)
and [restoration record](../reviews/issue5-error-factory/full-agents-result.json)
are retained. This closes the complete-file compile gap while preserving
the pending owner decision; full endpoint tests and workspace clippy were
not part of this candidate's compile experiment.

v14 check rejects only `type.impl_trait.ret` and the factory's `expr.closure`.
The checker needs definition-and-use recognition for this exact private
factory contract, with immediate std Result `map_err` consumers and no
other use. Broadly adding return `impl Trait` or allowing arbitrary returned
closures would exceed this rule. Name resolution remains a review limit,
as with the current closure-taking methods. No policy/config change is
included in this proposal.

Not measured: richer or mutable contexts, generic/stored/reusable factories,
other callback consumers, generators or other projects. Download/lock and
endpoint behavior does not change; no network installer test pass is claimed
by the reduced harness.

## Owner decision

Pending. This is a proposal, not an accepted subset construct.
