# Issue 5: comparisons, references, and remaining domain boundaries

Reviewed Conductor's **published** commit
`79568ce8a55b266fe1cf96b17dd5b2bc2dfa0c8c`, against subset
`64352fdd5935de3954ee06b4598789cb89f67a03`, `subset.toml` version 14.
The reference repository's working tree was read only. The archived commit,
not its potentially newer local `main`, supplied every measured source file.
The earlier `dac5e9c` reference is not the measurement baseline.

[inventory.txt](issue5-comparisons-references/inventory.txt) records the
searches at this commit: three reference workaround occurrences in two
items, two duplicated live-delta arms (each constructs its variant again),
two `MAX_FRAME_LEN` comparisons, and no explicit `-> bool` signature under
`crates`. Reconstructed historical round counts from the issue are not
treated as counts of still-existing friction sites.

No new construct qualifies for a proposal in this review. Four sites have
concrete rewrites using already accepted forms. Wider boolean patterns,
expression dereferencing, arbitrary early returns and nonactor domain
mutation do not follow from the owner's approval to consider issue 5.
No policy or checker source has been changed here.

## Existing forms that resolve the reported friction

The exact same real files before and after are under
[issue5-comparisons-references](issue5-comparisons-references/).
[supported-rewrites.diff](issue5-comparisons-references/supported-rewrites.diff)
contains all four rewrites.

| Published site | v14 form | Assessment |
|---|---|---|
| `crates/daemon/src/log.rs:104` and `:115` | Bind `kind @ (AssistantDelta { .. } \| ReasoningDelta { .. })`, then send `kind` once. | 0044/0048 already permit the wanted form; the two repeated bodies can be combined. |
| `crates/daemon/src/session.rs:77` | `find(\|&&offered\| offered == effort)` | 0053's shared reference patterns replace the reported `**offered`; comparison stays in the existing predicate closure. |
| `crates/daemon/src/registry.rs:245` and `:247` | Inside the `Some((provider, handle))` arm, `let &provider = provider;`, then pass `provider` directly. | Copy just the enum; keep the non-Copy session handle borrowed. This removes the two `to_owned()` calls without admitting `*provider`. |
| `crates/proto/src/frame.rs:25` and `:39` | `Ok(len @ ..=MAX_FRAME_LEN)` in `encode`, and `..=MAX_FRAME_LEN` in `body_len`. | 0049's integer ranges replace comparison against a numeric constant. The remaining arm still rejects greater lengths and failed conversion. |

The registry rewrite deliberately uses a separate shared-reference binding.
Destructuring an entire borrowed tuple would try to move the non-Copy handle;
an expression dereference is unnecessary. Rust ownership and 2024 binding
mode rules still decide which nested reference patterns compile.

`subset check` exits 0 on both sets of four complete files, with Conductor's
published config restricted only to these files. The unmodified published
Conductor snapshot also passed `subset check` in the initial inspection.
The exact scoped check output is retained in `before-check.txt` and
`after-check.txt`. No new tier was used: `pat.ident.at`/`pat.or`/`pat.range`
are encouraged (0.5), and `pat.reference` is neutral (1.0), with existing
boundary pricing unchanged.

## Measurements

Scores for functions below come from the full files with `--by fn`, so actor
ownership and boundary context are retained. The frame row includes both
functions, the unchanged named limit, and the removed `Ordering` import.
Extracted-function artifacts are additionally available, but a standalone
actor method loses its enclosing actor context and consequently has a
different absolute score. Its before/after saving is the same.

| Affected code | Score before → after | Saving | Tokens before → after | Tool lines before → after |
|---|---:|---:|---:|---:|
| `LogActor::on` | 239.5 → 196.5 | 18.0% | 266 → 222 | 52 → 42 |
| `choose` | 179.5 → 179.5 | 0.0% | 198 → 198 | 26 → 26 |
| `RegistryActor::on` | 409.5 → 408.5 | 0.2% | 439 → 438 | 65 → 65 |
| Frame functions, limit and import | 225.5 → 180.5 | 20.0% | 254 → 205 | 24 → 23 |

| Complete file | Score before → after | Tokens before → after | Tool lines before → after |
|---|---:|---:|---:|
| `log.rs` | 1222.0 → 1179.0 | 1257 → 1213 | 201 → 191 |
| `session.rs` | 4488.0 → 4488.0 | 4867 → 4867 | 723 → 723 |
| `registry.rs` | 2614.5 → 2613.5 | 2687 → 2686 | 417 → 417 |
| `frame.rs` | 1213.0 → 1168.0 | 1248 → 1199 | 207 → 206 |
| Total | 9537.5 → 9448.5 | 10059 → 9965 | 1548 → 1537 |

Log and range rewrites exceed the affected-code 15% test, but their syntax
is already accepted. The reference rewrites do not establish a new savings
case: the existing idiom is sufficient. No percentage of an isolated helper
is presented as a saving at its callers.

## Constants and runtime comparisons are different cases

0040 already allows a guard comparing an arm-bound value with another runtime
value, with a later unguarded covering arm. A lowercase local in a pattern
would bind a new value, not test equality with the existing runtime value.
Conductor's `session.rs:297` already uses the supported runtime form:

```rust
(Asked::Question { count }, Reply::Answer(answers)) if &answers.len() == count => Ok(()),
(Asked::Question { count }, Reply::Answer(answers)) => {
    Err(format!("{} answers for {count} questions", answers.len()))
}
```

A Rust constant with a structural pattern representation can instead appear
in a literal/path pattern, and integer/character limits now have 0049's
range form. `..=MAX_FRAME_LEN` expresses less-than-or-equal; a later fallback
expresses greater-than. Simply replacing `> MAX` with `MAX..` would include
equality and change behavior. Constants of unknown endpoint types remain
the documented review limit; arbitrary floating-point ordered predicates do
not gain permission from integer ranges.

The question probe in `s1.rs:588` compares with
`&Vec::from([Vec::from([2])])`. That is an allocation/call evaluated at
runtime, not a Rust const-pattern endpoint. Its existing guard passes v14.
Likewise, an allocated tuple of Strings is not made into a const pattern
merely because all string contents are literals.

`cmp` returning a three-state `Ordering` remains expressly allowed by 0024,
and examples §3 documents matching it. This review does not classify those
ordinary ordering matches as checker bugs. A boolean comparison by name,
such as `.cmp(...).is_eq()`, is a separate forbidden lift under 0040.

## `any`, `manual_map`, and boolean boundaries

The published `codex/protocol.rs:628` `served` function already searches with
`position` and matches `Some(_)`/`None`. That is a satisfactory named result
for the reported desire to guard with `.any()`. Existing predicate adapters
can call `.any()` inside their predicate, as in
`claude/session.rs:359`; a direct `.any()` guard is still a new guard shape.
Adding it has no measured qualifying case here.

The current `claude/protocol.rs:365` `grant` function builds its rules, then
uses `Some(rules).filter(|rules| !rules.is_empty()).map(...)` at `:382`.
This keeps the filtered result in the returned adapter chain rather than
matching a single-comparison lift immediately. It passes v14. Turning a
matched `Some(rules)`/`None` result into `Some(Grant)`/`None` is the ordinary
shape of clippy's `manual_map`; the allowed `.map(...)` already expresses
that mapping. There is no demonstrated need to disable the lint, and
0025's final owner decision expressly left all lints enabled.

The actual published function and its actual serde types were copied into
an isolated harness. `cargo clippy -- -D warnings` and the behavior run
both exited 0; [published-validation.txt](issue5-comparisons-references/grant/published-validation.txt)
retains that output. The run covers absent/empty suggestions, an ignored
mode change and a rule's label/session destination. This validates the
current implementation, not the anticipated Option-match lint diagnostic.

An additional existing-form alternative is matching `rules.as_slice()` as
`[] => None` and `_ => Some(Grant { ... })`. It avoids an Option mapping
match entirely. It is a suggested further check, not a validated rewrite
in this evidence set. `verify_grant.py` reconstructs the actual published
function, the slice alternative, and a literal Option-match reconstruction
that should reproduce `manual_map`, then runs clippy and behavior checks.
No anticipated diagnostic is presented as an observed error.

`client/answers.rs:92` already uses
`Some(answers).filter(|answers| answers.iter().all(|answer| !answer.0.is_empty()))`.
It returns the filtered value directly. The library boolean is confined to
the predicate; the domain output remains `Option<Vec<Answer>>`. Similarly,
`new_session.rs:66` `kept_effort` returns
`effort.filter(|effort| efforts(models, model).contains(effort))` and the UI
sets the resulting named `Option<Effort>`. Its helper expresses a reusable
domain decision without a domain bool.

The reported `use_signal(|| false)` became `use_signal(bool::default)` at
`app/session.rs:30`; the published form checks successfully. This one
boundary callback does not prove a case for domain bool return types or
patterns throughout ordinary modules. `bool` in external type arguments
and calls already has 0023's boundary permission; 0054 adds patterns only
in wire modules. `Some(true)` comparisons in model filtering and default
selection are already predicate expressions at `codex/protocol.rs:1066`
and `:1071`. Newly scoped wire patterns need not be substituted where an
existing collection predicate is already the intended form.

The credentials store's published `unseal` (`credentials.rs:108`) returns
`Option<Credentials>` through checked prefix/split and decryption, and the
credential API uses named errors/Results. A broad bool-signature rule would
weaken named states. No such proposal is filed.

## toml_edit and nonactor drafts

`codex/config.rs:34` `with_provider` uses the library's `DocumentMut`, `Table`
and `insert`/`remove` APIs, matching credential presence and returning the
result at the tail. `section` at `:103` creates an implicit library table
and returns `Option<&mut Table>`; external mutable Table APIs already fit
0008/0023. The library-required `set_implicit(true)` is a permitted boundary
literal. Explicit indexing and early `return` are still outside the subset,
but the real implementation has working existing forms. General indexing
can panic and arbitrary return would hide an exit; neither has a measured
seven-quality case here.

`client/answers.rs:32` `Draft::pick` and `:57` `Draft::type` already consume
`self`, destructure its library Vec fields into mutable local bindings, and
rebuild a `Draft`. Only library collections mutate. Changing these methods
to `&mut self` on a nonactor domain draft would weaken forward data flow
and reopen 0008's explicit rejection of arbitrary mutable domain values.
This is not a checker bug or an implied stream-endpoint extension under
0035. No broader mutation proposal is filed.

## Seven qualities and remaining verification

All four supported rewrites retain the seven qualities:

1. Named states: the same event, provider, effort and typed error models remain.
2. Visible branching: the log alternatives and numeric partition appear in patterns; no hidden early exit is added.
3. Forward data flow: copies and shared destructuring add no mutation.
4. Errors are values: the frame failure arms and registry Result mapping remain.
5. Local reasoning: selected payloads and discarded fields are visible at the use.
6. One way: the already approved structural forms replace duplicate spellings.
7. Repetition is data: the two live-event bodies become one body with explicit variant alternatives.

No actionable deviation from an accepted checker rule was confirmed in this
scope. The direct `.any()` guard remains unadmitted policy; accepting it is
not a checker repair. Unknown type/provenance restrictions remain the
documented review limits.

The four complete after files were checked and measured with the published
checker. Compilation of these exact rewrites is delegated to the consumer
implementation round, which runs the actual Conductor gate. This report
does not claim a cargo/clippy pass from syntax checks. Shared WSL launch
failures (`Wsl/Service/WSAETIMEDOUT`) delayed independent grant validation;
the previously launched job later completed the published-form check, with
the slice/manual-map checks still outstanding when this report was written.
At the coordinator's request, new launches were paused to preserve the
already running consumer gate. No subsystem or unrelated process was
terminated. Other projects and unpublished Conductor changes were not measured.

To reproduce the retained numbers in an environment with WSL/Linux and the
v14 checker available:

```sh
SUBSET=/path/to/v14/subset \
  bash docs/reviews/issue5-comparisons-references/reproduce.sh
```

The script archives the exact published commit into this project's `target`
and invokes `subset score --by file`, `subset score --by fn`, and
`subset measure` on the same before/after paths. Full commands, output and
exit status are retained. `CONDUCTOR_REFERENCE` overrides the read-only
reference repository path. This is consumer evidence for existing v14
forms, not a proposed or accepted expansion of the subset.
