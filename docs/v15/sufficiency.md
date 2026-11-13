# Is version 15 sufficient to write the checker?

The benchmark: the checker's own source, `src/`, must be writable in the
language of [`spec.md`](spec.md) without dropping behaviour. This page
records a first trial against that benchmark, made on 2026-11-13, and the
gaps it found. It is evidence for the owner, not a ruling.

## Method

1. **Trial rewrites.** Two pieces of `src/` were rewritten by hand in
   version 15, in a scratch copy, and run through the full test suite:
   - `src/measure.rs`, the whole module (a small one: lexing, doc-comment
     skipping, line counting);
   - the actor-loop predicates in `src/checks.rs` (`turn`, `takes_input`,
     `handles`, `select`, `awaits` and their helpers), the densest
     `Option<()>` code in the checker.

   The before and after files are in [`trial/`](trial/).
2. **A census.** Line-pattern counts, over `src/` and Conductor's
   `crates/`, of every pattern the trial showed to be awkward or missing a
   form. The counts are estimates and upper bounds.

## Result

Both rewrites build, and every behaviour test passes: all 94 cases, the
compiled pattern tests, the example check and the HTML report test. The one
failure is `the_checker_passes_its_own_subset`, which is expected, because
the version 14 checker flags version 15 code (`if`, ranges, `bool`).

| Piece | Before | After | Change |
|---|---|---|---|
| `measure.rs` | 753 tokens, 92 lines | 918 tokens, 122 lines | +22% tokens, +33% lines |
| loop-turn predicates | 543 tokens, 60 lines | 479 tokens, 47 lines | −12% tokens, −22% lines |

**Verdict: sufficient, with three gaps that need a ruling and one
exemption.** Nothing the trial touched was impossible to express, but in
three places the only available form is either a second way to do a job or
something the spec forbids. Without rulings on those, package 9 would stall
at the same points.

## Gaps that need a ruling

### G1. Building pairs for a library that takes pairs

`collect` into a `BTreeMap` or `HashMap`, and `BTreeMap::from([(k, v), ..])`,
take tuples. The spec forbids building a tuple as a value, so neither has a
form. The only legal alternative is
`let mut map = BTreeMap::new(); for x in list { map.insert(k, v); }`,
which is a second form for "collect" and reads worse.

| | Checker `src/` | Conductor |
|---|---|---|
| `collect::<…Map>` | 6 | 3 |
| `Map::from([(..)])` | 0 | 5 |
| closures that build a tuple, `\|..\| (a, b)` | 21 | 20 |

**Recommendation:** this is the library-boundary rule applied to tuples.
A tuple may be built as the item a library consumes as a pair: the closure
directly before `collect` into a map, or an element of a `Map::from([..])`
literal. It is never bound, stored or returned.

### G2. Ordering by a composite key

The checker ranks rows by `(Reverse(count), name)`: a tuple key in a
`BTreeMap`, or `min_by_key` / `max_by_key` returning a tuple. With tuples
banned the key must be a named struct, but a named struct cannot derive
`Ord`: carried-over proposal 0024 allows `Ord` only on a newtype.

| | Checker `src/` | Conductor |
|---|---|---|
| `Reverse(..)` in a key | 4 | 1 |

**Recommendation:** allow `PartialOrd` and `Ord` on a named struct whose
only use is as an ordering key, with fields in priority order. This is
small, mechanical (the derive is on the struct) and states the order in
names.

### G3. A predicate that must say "no" in some case

A predicate fn returns `bool`, but `true` and `false` are banned, so a
predicate has no way to answer "no" from a fallback case:
`None => false`, or the length cases of a slice match. The trial's
`is_turn` was:

```rust
match body.stmts.as_slice() {
    [take] => takes_input(take),
    [take, handle] => takes_input(take) && handles(handle),
    [] | [_, _, _, ..] => false,      // not allowed
}
```

The only literal-free form is `matches!` with a guard, chained:

```rust
matches!(stmts, [take] if takes_input(take))
    || matches!(stmts, [take, handle] if takes_input(take) && handles(handle))
```

That form is readable here, but the carried-over rule `MatchesForm`
forbids an `if` inside `matches!`, and `is_some_and` (the other way to
write "present and satisfying") is removed by `choose.method`. So as
specified, no form is left.

Eight of the checker's 77 `Option<()>` fns use `?` internally, and every
one of them meets this when it becomes a `bool` predicate.

**Recommendation:** make `matches!(value, Pattern if condition)` the one
form for "this value has this shape and satisfies this", and drop the
no-guard clause from `MatchesForm`. The alternative, allowing `true` /
`false` as arm values inside a predicate fn, contradicts the owner's
"never".

### G4. Names a library chooses

syn's `Visit` fixes method names such as `visit_impl_item_fn` (4 words),
so `name.words` cannot be met inside that impl. The same applies to any
trait impl.

**Recommendation:** the shape limits on names and parameters do not apply
to the signature of a method in a library trait impl. Length and depth
still do.

## Costs that are not gaps

These have a form; they make code longer and the owner should know before
package 9.

- **Exhaustive matches.** `measure.rs` grew mostly by naming every variant
  instead of `_` and by one named struct (`Placed`) in place of a nested
  tuple. Across `src/`, about 60 matches over our own enums use `_`, 40 of
  them two-arm "take one variant" matches inside `filter_map`. Each grows
  by one or more alternatives.
- **Tuple accumulators.** 19 `fold`s in `src/` carry a tuple. Each needs a
  named accumulator struct.
- **One-expression closures.** 101 closures in `src/` and 52 in Conductor
  have a block body. Each becomes a named fn, and captured context becomes
  parameters, which can hit the 4-parameter limit.
- **`Option<()>` predicates.** 77 in `src/`. The trial shows these
  get shorter, not longer, as `bool` predicates (−12%).

Whether +22% on `measure.rs` is acceptable is a principle 1 judgement:
the after version names every case and every intermediate, and has no
positional data. It is longer and, on reading, plainer.

## Smaller clarifications for the spec

- **Shadowing and fn names.** `let sources = sources(root, &files)?;` binds
  a local with the name of a fn. The rule should say whether only local
  bindings count. Recommendation: only bindings shadow bindings.
- **Macro bodies.** `select!` arms are not parsed, so none of the version 15
  rules apply inside Conductor's 11 `select!` uses. This is the existing
  documented limit; it is worth restating in the spec.

## What this trial did not cover

- About 4% of `src/` was rewritten. `walk.rs`'s visitor and the large
  `checks.rs` dispatchers, which the shape limits will hit hardest, were
  not.
- Conductor's async and actor code was only counted, not rewritten. Its
  benchmark is package 11.
- The counts are line patterns. Package 1's `report --pending` gives real
  numbers.

The full benchmark is package 9 ([#33](https://github.com/JimLundin/subset/issues/33)).
It should start with the rulings above in place.
