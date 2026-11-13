# Issue #5: platform attributes and test re-exports

This review uses published Conductor commit
`79568ce8a55b266fe1cf96b17dd5b2bc2dfa0c8c`, its `subset.toml` version 14,
and subset baseline `64352fdd5935de3954ee06b4598789cb89f67a03`.
The Conductor checkout was read only; its unpublished local changes were
not evidence. The older issue sandbox at `dac5e9c` was not the baseline.

The actionable result is a checker correction: complementary attributes
on unrelated functions, methods, or impls must not count as counterparts.
This change adds no syntax and does not change version 14. Neither
`target_os` attributes nor a broader test visibility rule are accepted by
this review.

## Existing decisions and inventory

[Proposal 0016](../proposals/0016-attr.cfg.md) admits exactly `unix`,
`not(unix)`, `target_family = "wasm"`, and its negation. Attributes belong
on `mod`, `use`, `fn`, or `impl` items. The item has its counterpart in the
same file; test files and modules that exist on one platform are exempt.
The explicitly accepted test-file root form is `#![cfg(unix)]`.

`#[cfg(test)]` is a separate accepted ID, `attr.cfg_test`, as are `#[test]`
and the async test attribute from [0020](../proposals/0020-attr.tokio.test.md).
It is not required to have a platform complement. The attribute walker
already classifies it separately and skips the platform predicate check.
[0002](../proposals/0002-vis.pub.md) reserves `pub` for crate API;
[0003](../proposals/0003-vis.restricted.md) admits `pub(crate)` for internal
sharing. Neither requires extra test export syntax.

The feature-gated fixture considered in [p1-u6](p1-u6.md#the-fixture) was
rejected because it needed additional build modes, combinations, and
variant/arm gating. This review neither reopens that rejection nor admits
`all`, `any`, feature predicates, `cfg_attr`, or expression/statement
platform attributes.

`inventory.json` records every published source line found by:

```sh
git grep -n -F -e '#[cfg(' -e '#![cfg(' -e 'consts::OS' -e 'check_version' \
  79568ce8a55b266fe1cf96b17dd5b2bc2dfa0c8c -- crates
```

There are 14 platform attributes (12 item attributes and two Unix-only
integration test roots), 21 `cfg(test)` attributes, and zero `target_os`
attributes. The platform items are in `agents/src/install.rs`,
`agents/src/process.rs`, `client/src/platform/mod.rs`, and
`daemon/src/lib.rs`. The two integration test files are
`agents/tests/replay.rs` and `agents/tests/codex_replay.rs`.

## Required checker fix: counterpart identity

The baseline collected a function's bare name, an impl's last type name
and trait path, and one empty name for every use. It searched all matching
facts in the file. Consequently, this complete minimal example passed
with exit 0 and no findings:

```rust
mod first {
    #[cfg(unix)]
    fn socket() -> Option<u8> {
        Some(1)
    }
}

mod second {
    #[cfg(not(unix))]
    fn socket() -> Option<u8> {
        None
    }
}
```

These functions do not offer two platform implementations of one item:
they belong to different module APIs. The complete regression fixture
also reproduces methods on `First` and `Second`, nested functions in
different bodies, and whole impls for `owner_first::Port` and
`owner_second::Port`. All eight attributes passed the baseline.

The correction retains kind, name, and exact predicate complement and
adds the containing inline module path, nearest ordinary block identity,
and logical impl owner. The latter compares normalized syn type and trait
paths, retaining concrete arguments and distinguishing local `crate::`
roots from external `::` roots. A preliminary syntax walk gathers module,
block, declaration, and ordinary import bindings before both checker
passes. Lookup uses the current module and visible enclosing blocks;
explicit `self`, `super`, and `crate` prefixes resolve against that scope.
An ordinary named import follows its recorded target, with a bounded
lookup to avoid cycles. Unrelated sibling and block-local declarations
cannot change external root interpretation. Qualified `crate`, `self`, and
`super` lookup begins in the actual module scope, excluding function-local
shadows. Local declarations retain their defining block origins in the
owner identity. Both bare and qualified paths walk intermediate imported
module bindings. Bound type/lifetime names normalize separately;
a bound type variable uses a private marker that cannot equal a concrete
Rust type path. Token spans do not affect identity. This distinguishes
different types and traits while preserving compatible spellings and
separately written impl blocks for the same owner. Binder identities use
first occurrence across the self type and trait path, so reordering blanket
impl binders preserves identity while repeated-parameter constraints remain
distinct.

Positive regression coverage retains root and inline-module function
pairs, nested pairs in the same body, methods split across inherent impls
for the same type, paired whole impls, alpha-renamed generic/lifetime
parameters, relative/absolute root spellings, imported parent types, and
external imported types/traits. Existing anonymous use pairing
and the platform-only module exemption remain unchanged within their
container. `native` and `web` module names intentionally differ in the
accepted code; this review does not impose equal module names.

The policy has not changed: the checker now identifies the item whose
other implementation the accepted rule requires. Savings and the
three-friction-point threshold govern new constructs, not corrections
that enforce accepted policy. The fixture and verbatim before/after
outputs are included in the reproducible evidence.

All 28 attributes in the negative fixture now produce missing-counterpart
findings; the baseline produced none. The positive fixture produces none.
It contains 42 allowed platform attributes plus one test-only re-export.
The separate 16-case matrix confirms the predicate and host boundaries:

| Form | Observed checker result |
|---|---|
| Unix or wasm function pair | Pass |
| Missing function counterpart | Reject as `CfgPair` |
| `target_os`, feature, `all`, double negation | Reject as `CfgPredicate` |
| Statement, expression, struct, production file-root attribute | Reject as `CfgHost` |
| Unix test-file root or unpaired item inside a test file | Pass under the existing exemption |
| Platform-only module or test re-export | Pass |
| Wasm test-file root | Pass: existing checker breadth exceeds 0016's specifically named Unix root form |

The last row is an existing documentation/enforcement discrepancy, outside
this counterpart correction. It is recorded rather than silently
accepting new syntax or narrowing existing test behavior. The broad
module exemption remains a review check for whether the module actually
exists only on one platform.

## OS-specific attributes: measured, not accepted

The reported site is
`crates/agents/src/claude/session.rs:354` (`sandbox`) at the published
commit. The complete before item is in
`issue5-platform-attributes/sandbox-linux-pair.before.rs`; the recorded
snippet also includes its `Sandbox` enum, tools constant, and import.
It matches `std::env::consts::OS` and the first missing tool. macOS is
always available; Linux needs both `bwrap` and `socat`; other OS values
are missing.

`std::env::consts::OS` itself is a compile-time target constant, not a
runtime OS query. The current function still performs the generic PATH
and file checks before its source-level match.

Wanted syntax was `cfg(target_os)`. The candidate rule considered here is
narrower: allow a `target_os = "linux"` function and its exact `not`
counterpart in the same logical container, retaining all existing host
and pair requirements. The Linux function checks the tools and matches
`Option`; the complement matches the OS for the macOS case. Its constant
moves inside the Linux function so other targets do not acquire an unused
constant. The diff applies to the exact published source.

`subset check` rejects the new predicates as `attr.cfg` under 0016. The
proposed tier is **tolerated**, weight 2.0, matching existing compile-time
platform branches. Boundary pricing does not apply: this is our platform
selection policy, rather than a library-prescribed signature. The JSON
includes raw outside pricing and the explicit arithmetic that replaces
5.0 with 2.0 for the attribute tokens.

| Sandbox scope | Before score / tokens / measured lines | After score / tokens / measured lines | Score change |
|---|---|---|---|
| Affected declarations and import | 157.0 / 168 / 18 | 192.5 / 199 / 26 | +22.61% |
| Whole session file | 4206.0 / 4363 / 715 | 4241.5 / 4394 / 723 | +0.84% |

The affected after score is `240.5 - (5.0 - 2.0) * 16 = 192.5`.
The pair preserves all seven qualities but increases the score and has
only one friction point. It does not qualify for a proposal. Keep the
published runtime match under the currently accepted rule.

There is one sandbox friction point. Existing `platform()` in
`agents/src/install.rs:67` uses an OS/architecture match to map six vendor
names with an explicit unpinned error fallback. It is already a compact
data mapping and is not an additional measured sandbox friction point.
`ClientName` in `client/src/platform/native.rs:8` uses the OS name as data,
also not evidence for compile-time branching.

A related robustness check is `agents/src/process.rs:13`: `not(unix)`
guards `std::os::windows::process::CommandExt`. Windows is not the entire
non-Unix target set. A Windows-specific predicate and its negation would
be a more accurate library gate if supporting non-Unix, non-Windows
targets became a requirement. This is not an established bug in
Conductor's supported Linux/macOS/Windows native host build: its current
gate is correct on those hosts, and the agents crate's other dependencies
also limit its target set. The exact rewrite is measured separately. It
does not add a second sandbox friction point or justify silently widening
0016.

| Windows gate scope | Before score / tokens / measured lines | After score / tokens / measured lines | Score change |
|---|---|---|---|
| Affected function pair and import | 92.5 / 81 / 12 | 100.5 / 85 / 12 | +8.65% |
| Whole process file | 224.0 / 223 / 28 | 232.0 / 227 / 28 | +3.57% |

The affected after score is `148.5 - (5.0 - 2.0) * 16 = 100.5`.
This single exploratory robustness site also fails the proposal threshold.

## `check_version` test re-export: already supported

The whole module item at `agents/src/claude/mod.rs:9` is:

```rust
#[cfg(test)]
pub(crate) use protocol::check_version;
```

Its consumer is the test module import in `agents/src/install.rs:374`,
called by the complete test
`claude_pin_is_compiled_recorded_and_supported` at line 435. Production
session code already imports the private protocol function directly.
The test-only re-export admits one function to sibling tests without
exposing the whole protocol module or adding a production-only unused
import. It passes subset v14.

The alternative measured in `direct-test-module-access` makes
`protocol` a `pub(crate)` module and changes the test import to
`crate::claude::protocol::check_version`. That is existing syntax, not a
proposal. It exposes the protocol module's internal surface to the whole
crate solely for one test. A small affected import score is not evidence
that this broader surface is desirable. The existing narrow re-export
is the recommendation; no checker change is required for it.

| Test visibility scope | Before score / tokens / measured lines | After score / tokens / measured lines | Score change |
|---|---|---|---|
| Changed declaration, re-export, and test import | 26.0 / 26 / 4 | 18.0 / 18 / 2 | -30.77% |
| Whole module and install files | 3694.0 / 3684 / 494 | 3686.0 / 3676 / 492 | -0.22% |

The eight-point saving is available within the accepted subset. It does
not establish a missing construct, and this alternative was not compiled
as a complete Conductor change. The narrower current module surface is
preferred. The one test site cannot be pooled with platform attribute
sites toward the three-friction-point threshold.

## Qualities

| Quality | Counterpart correction | Linux pair candidate | Existing test re-export |
|---|---|---|---|
| Named states | Kept: item identity is explicit. | Kept: outcomes remain `Sandbox`; missing tools remain `Option`. | Kept: no state representation changes. |
| Visible branching | Kept: unrelated items cannot hide the absent platform arm. | Kept: exact item complement and explicit runtime fallback. | Kept: test availability is an explicit item attribute. |
| Forward data flow | Kept: checking changes no application data. | Kept: all values remain immutable. | Kept: visibility adds no mutation. |
| Errors are values | Kept: no application error behavior changes. | Kept: the availability result remains a named enum. | Kept: `check_version` still returns `Result`. |
| Local reasoning | Kept: each counterpart belongs to its actual container. | Kept: each platform implementation is a complete function. | Kept: the module explicitly exports just the test helper. |
| One way | Kept: existing pairs retain their existing form. | Kept if accepted: use `target_os` for this OS-specific item pair; keep OS names used as data. | Kept: existing `pub(crate)` item sharing idiom. |
| Repetition is data | Kept: no application shapes are duplicated. | Kept: tool values remain one constant list and one scan. | Kept: the existing protocol implementation stays shared. |

## Reproduction and limits

From this review worktree on Linux/WSL:

```sh
python3 docs/reviews/issue5-platform-attributes/measure.py \
  --conductor /home/jim/projects/apps/conductor \
  --checker "$PWD/target/debug/subset" \
  --baseline-checker /mnt/c/Users/Jim.Lundin/Projects/subset/target/debug/subset \
  --compile
```

The script uses only `git show`/`ls-tree` on the published commit, writes
all generated source and compilation output under this worktree's
`target/issue5-platform-evidence`, and retains exact affected snippets,
whole-file diffs, inventory, checker output, scores, token/line measures,
and repricing arithmetic beside this report.

The retained diffs use zero context to keep patch-context blank lines from
being interpreted as trailing source whitespace; use `git apply
--unidiff-zero` if applying them directly. The script recreates the same
rewrites without patch application.

Sandbox compilation checks the actual rewritten functions natively and
as a wasm library. Native subprocesses check empty PATH, only `bwrap`,
only `socat`, and both tools, using ordinary files as the function itself
requires; no command is run. Other OS runtime cases, a complete Conductor
workspace gate, and the Windows branch's actual target compilation are
outside this isolated measurement. No saving for those checks is claimed.
The no-window and test-surface rewrites are exploratory evidence, not
implemented Conductor changes. Macros are not expanded and Rust name
resolution is limited to recorded ordinary imports and syntactically
visible declarations in these files. Glob imports, conditional import
ambiguities, macro-defined types, out-of-file module exports, associated
type projections, and equivalence requiring compiler type analysis remain
review/compiler checks. Generic normalization preserves first-occurrence
binder correspondence and repeated-variable constraints; it does not prove
equivalence of arbitrary generic bounds or compiler-resolved types.

Whole-file scores use the listed files' syntax/name context, not all other
Conductor files; no whole-project saving is claimed. `subset measure`
counts nonempty code lines and ignores comments, so its lines are not
physical source-file line numbers.

Validation of the checker change: `cargo fmt --all`, `cargo test --offline`
(all 11 integration test groups, including all 95 CLI case directories), `cargo clippy --offline --all-targets --
-D warnings`, and `subset check` pass. The evidence script compiles both
actual sandbox rewrites natively and for wasm and confirms the four native
PATH results are identical. This repository has no `tools/check.sh`; the
Conductor full gate was not run for these exploratory rewrites. The added
compiler group extracts the actual self-contained positive fixture items
for imported module owners, local generic owners, and reordered binders;
those items compile on the native target. The cfg fixture retains explicit
negative cases for local/global shadows and repeated-parameter versus
blanket generic owners.
