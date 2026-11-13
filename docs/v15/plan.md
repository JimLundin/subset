# Implementing version 15

How to take the checker, its own source and Conductor from version 14 to the
language in [`spec.md`](spec.md). The rulings are in
[issue #24](https://github.com/JimLundin/subset/issues/24); each work package
below has its own issue under it.

## Read first

- [`spec.md`](spec.md): the target. It is normative; this page is not.
- `src/table.rs`: every rule the checker has today (`Rule`, `RULES`, `IDS`).
- `src/walk.rs`: the one walk over a file (`Walk`, an implementor of syn's
  `Visit`). It emits ids, findings and facts.
- `src/checks.rs`: the shape rules, one fn per rule or family.
- `tests/cli.rs` and `tests/cases/`: one directory per case, `expected`
  holds the output, `SUBSET_BLESS=1 cargo test` rewrites it.

## Constraints

- **The checker is written in the subset it checks.** The test
  `the_checker_passes_its_own_subset` runs `subset check` on `src/`. A new
  restrictive rule would fail it the moment it lands, so every such rule
  lands *pending* (below) and is switched on when `src/` complies.
- **The gate passes on every commit:** `cargo fmt --check`,
  `cargo clippy --all-targets -- -D warnings`, `cargo test`.
- **Every rule has a case.** A rule lands with a directory under
  `tests/cases/` holding a passing file and an `a_fail.rs`, as the existing
  cases do.
- **The README tables are generated.** `subset ids` and `subset rules` must
  match `README.md`; a test checks it.
- **Diagnostics state the fix** in at most about 15 words, as in
  `` `else if`: write a `match` ``. Rule text no longer cites proposal
  numbers.
- **Code changes go through the pair loop** (`.claude/skills/pair/SKILL.md`)
  in both repositories.
- **Open no pull request on Conductor** unless the owner asks.

## The pending mechanism

Version 15 adds rules that today's code breaks in hundreds of places. To
land them one at a time without a long-lived branch:

```toml
# Rules the checker knows but does not yet enforce here.
pending = ["name.shadow", "fn.lines"]
```

- A finding of a pending rule does not fail `check` and is not printed by
  it.
- `subset report --pending` lists pending findings by rule, with counts.
- A rule leaves `pending` in the commit that brings its count to zero.
- `version = 15` is set when `pending` is empty, and the key is then
  removed from the config schema. A config with `version = 15` and a
  `pending` key is an error.

This is a design choice of this plan, not an owner ruling. It is the first
thing work package 1 builds.

## Rule inventory

### Rules to add

Ids are proposals; keep them short and stable once landed.

| Id | Rule | Sites today (checker `src/`, Conductor `crates/`) |
|---|---|---|
| `name.shadow` | a name is not rebound while an earlier binding is in scope | ≤11, ≤86 |
| `fn.depth` | block nesting inside a fn is at most `limits.depth` | not counted |
| `fn.lines` | a fn is at most `limits.lines` lines unless a dispatcher | 48, 85 before the dispatcher exemption |
| `fn.params` | at most `limits.params` parameters besides `self` | 6, 8 |
| `name.words` | a name has at most `limits.words` words and no `and` / `or`; tests exempt | 5, 4 |
| `closure.block` | a closure passed to a method is one expression | not counted |
| `match.catch_all` | no `_` arm in a `match` over an enum of ours | up to ~370 in `src/` |
| `tuple.ours` | no tuple type, and no tuple value outside a `match` subject | 9 and 14 fns return one |
| `field.position` | `.0` only inside a newtype's own methods | ≤86, ≤220 |
| `state.clone` | a type with `&mut self` methods does not derive `Clone` | 0, 1 (`Session`) |
| `assign.field` | assignment only to `self.field` in a `&mut self` method | already true |
| `assign.compound` | `self.x = self.x + n` is written `self.x += n` | new form |
| `bool.position` | `bool` only as a predicate fn's return type, or in a boundary type | replaces `BoolType` |
| `bool.literal` | `true` / `false` only as a `const` initializer | replaces `BoolLit` |
| `bool.bound` | a boolean expression is not bound to a name | replaces the `Predicate*` rows |
| `predicate.name` | a fn returning `bool` has a name with a prefix in `predicate_prefixes` | new |
| `if.chain` | no `else if` | new |
| `if.value` | an `if` whose value is used has an `else` | new |
| `choose.method` | a method in `branch_methods` is a `match` or an `if` | ~105, ~25 |
| `choose.unit` | `Option<()>` is not a type of ours | 76, 0 |
| `choose.default` | the argument of `unwrap_or` is a literal, a constant or a path | not counted |
| `guard.only` | a `match` that only tests a comparison is an `if` | not counted |
| `fail.unwrap` | a method in `test_only_methods` only in test code | 0, 0 in production |

Counts are line-pattern estimates from the review and are upper bounds.
`subset report --pending` gives the real ones once a rule exists.

### Rules to remove

| `Rule` variants | Why |
|---|---|
| `OperatorUse`, `OperatorPath`, `OperatorType`, `OperatorMethod` | arithmetic is in the core |
| `RangeCall`, `RangeStruct`, `RangePath` | a range is written `a..b` |
| `SplitAt` | freedom from panics is not a goal (confirm with the owner, below) |
| `ForBody`, `ForIter` | redundant without mutable locals |
| `MutSelf`, `RefMutSelf`, `LetMutOurs`, `ParamMutOurs`, `RefMutOurs` | replaced by `assign.field` and `state.clone` |
| `FromOnce` | `attr.from` leaves `allowed` |
| `StrTupleRow`, `TupleTable` | replaced by `tuple.ours` |
| `LiftFilter`, `LiftMatched` | replaced by `choose.method` |
| `PredicateEq` … `PredicateNot`, `PredicateMatches` | replaced by `bool.bound` |
| `BoolType`, `BoolLit` | replaced by `bool.position`, `bool.literal` |
| `OrdInner` | generalised by `field.position` |

With them go the `fallible_operators` and `endpoints` config keys, the
loop-state ownership facts in `src/facts.rs`, and the `Visit` exception.

### Rules to keep, and what changes

| `Rule` variants | Change |
|---|---|
| `Guard`, `GuardBound`, `GuardPattern`, `GuardFallback` | keep; the condition may now use `&&`, `\|\|`, `!` |
| `LiftEq` (comparison by name) | keep, as the one-form rule for `==` |
| `ForEach`, `StreamEffects`, `Count`, `MatchesForm` | keep |
| `LoopPosition`, `LoopTurn`, `Break` | keep |
| `LetMut`, `PatMut` | keep: `mut` is only lent, never reassigned |
| `Assign` | becomes `assign.field` |
| `ImplTrait`, `Closure`, `ClosureMethod`, `AsyncBlock`, `Dyn`, `Glob`, `Allow`, `Env`, `TokioMain`, `TokioTest`, `ValueParser`, `ImplTraitArg`, `SerdeKeys`, `SerdeDefault`, `SerdeOther` | keep the rule, move every library name it matches into config (below) |
| the rest (`VisCrate`, `PubField`, `StructUpdate`, `UnitType`, `UnitValue`, `DropDiscard`, `DropFn`, `OkDiscard`, `Reader*`, `Lifetime*`, `StaticStr`, `KeyedTable`, `ThiserrorHost`, `OrdHost`, `ErrorMessage`, `TypeParam`, `Slice`, `ArrayType`, `ArrayValue`, `Repeat`, `Cfg*`, `ClosureParam`, `IdentAt`, `SliceRest`, `SharedPattern`, `SourceField`, `IncludeStr`, `RangePattern`) | unchanged, apart from shorter messages |

### Ids that change in `allowed`

- **Added:** `expr.if`, `expr.binary.{add,sub,mul,div,rem}`,
  `expr.unary.neg`, `expr.range`, `expr.binary.{add,sub,mul,div,rem}_assign`.
  The comparison and boolean operator ids are already there.
- **Removed:** `attr.from`. `expr.tuple`, `pat.tuple` and `type.tuple` stay
  listed and are confined by `tuple.ours`.
- **Stays out:** `expr.index`, `expr.while`, `expr.continue`, `expr.return`,
  `stmt.let.else`, `expr.cast`, `expr.if_let` with an `else`.

## Config schema

New keys, each with today's hard-coded value as its default so Conductor's
behaviour does not change when the names move out of the source:

```toml
pending = []                       # transitional, see above

[limits]
depth = 3
lines = 40
params = 4
words = 3

predicate_prefixes = ["is", "has", "can", "should"]
branch_methods = ["map_or", "map_or_else", "unwrap_or_else", "is_some_and",
                  "is_none_or", "is_ok_and", "is_err_and", "then", "then_some"]
test_only_methods = ["unwrap", "unwrap_err"]

# Library names that are in the source today:
library_traits = ["Drop", "Debug", "Serialize", "Deserialize", "ProtocolHandler", "Visit"]
closure_callees = ["unfold", "use_signal", "use_future", "use_hook", "ask"]
async_block_callees = ["spawn", "unfold", "then", "use_future"]
closure_wrappers = ["Closure::new"]
glob_imports = ["dioxus::prelude"]
flag_derives = ["Parser", "clap::Parser"]
env_names = ["CARGO_PKG_VERSION", "CARGO_PKG_NAME", "CARGO_MANIFEST_DIR"]
interior_mut = ["Cell", "RefCell", "OnceCell", "OnceLock", "UnsafeCell", "Mutex", "RwLock", "Atomic*"]
```

Removed keys: `fallible_operators`, `endpoints`. The config is validated as
in issue #8: unknown ids and rule names are errors, and `version` is an
integer the checker compares with its own.

`[weights]` shrinks to the overlaps in the spec. Every id is neutral unless
it is the less-preferred side of an overlap (`fold`, `first` / `last`, a
nested choice where a guard fits) or a primitive where a newtype belongs.
The boundary share and the typing columns stay as reports.

## Work packages

Each is an issue under #24. The order is the dependency order; 4 to 7 can
run in parallel once 3 has landed.

| # | Package | Depends on | Done when |
|---|---|---|---|
| 0 | Gate: CI and a dated toolchain (issues #10, #11) | | the gate runs on every push |
| 1 | Foundations: `pending`, `[limits]`, config validation and the version constant (#8) | 0 | `report --pending` works; a bad id or version is exit 2 |
| 2 | Library names into config (#13) | 1 | no library name in `src/`; Conductor's output is unchanged |
| 3 | Returned constructs and removed rules | 1 | the ids above are allowed; the removed rules and keys are gone; cases updated |
| 4 | Rules: names and state | 3 | `name.shadow`, `assign.*`, `state.clone` exist with cases, pending |
| 5 | Rules: choose | 3 | the `bool.*`, `if.*`, `choose.*`, `guard.only`, `match.catch_all`, `predicate.name`, `fail.unwrap` rules exist with cases, pending |
| 6 | Rules: data | 3 | `tuple.ours`, `field.position` exist with cases, pending |
| 7 | Rules: shape | 3 | `fn.*`, `name.words`, `closure.block` exist with cases, pending |
| 8 | The score as a tie-breaker | 3 | `[weights]` holds only the overlaps; the report shows them |
| 9 | Rewrite `src/` and empty `pending` | 4–8 | `pending = []` here; `version = 15`; tagged `v15` |
| 10 | Spec and docs | 9 | `spec.md` is `docs/subset.md`; `examples.md` rewritten and tested; history in a changelog (#22, #17) |
| 11 | Conductor | 9 | Conductor pins `v15` with its own `pending` list, then empties it |

Suggested staffing: one pair on 0–3 in sequence, then one pair per package
for 4–8, then all on 9. Package 11 runs in Conductor's repository under its
own pair loop and can start as soon as 9 is tagged.

### Order inside package 9

Rewrite by rule, cheapest first, removing each from `pending` as it reaches
zero: `state.clone`, `fn.params`, `name.words`, `name.shadow`,
`assign.compound`, `choose.unit`, `bool.*` and `predicate.name` (these
three turn `Option<()>` predicates and `Within` flags into predicate fns
and named enums, and should go together), `choose.method`, `if.*`,
`guard.only`, `tuple.ours`, `field.position`, `match.catch_all`,
`closure.block`, `fn.depth`, `fn.lines`. The shape limits go last because
every earlier rewrite changes fn sizes.

## Details the rulings left open

Each has a default so work is not blocked. Confirm with the owner before
the package named.

| Detail | Default | Package |
|---|---|---|
| Bitwise operators (`& \| ^ << >>`) | return with arithmetic | 3 |
| `split_at` | allowed again | 3 |
| `ok_or_else`, `unwrap_or_default`, `or_else` | allowed: same jobs as `ok_or` and `unwrap_or` | 5 |
| A boolean bound to a local, `let ready = a < b;` | not allowed (`bool.bound`) | 5 |
| Prefixes that make a predicate name | `is`, `has`, `can`, `should` | 5 |
| `first()` / `last()` beside slice patterns | allowed, less preferred in the score | 8 |
| A `bool` field on a serde wire type | allowed where the project's config scopes serde | 5 |
| Whether a dispatcher's arm may be a block of one call | yes | 7 |
| How lines are counted for `fn.lines` | from the `fn` line to its closing brace, comments included | 7 |

## What is out of scope

- New constructs beyond the rulings. Friction found during the rewrite is
  logged and goes through `docs/presenting-a-case.md`.
- The tooling issues #7, #9, #12, #15 and #16. They are independent and can
  be done at any time.
