---
construct: expr.array, pat.slice, pat.rest
status: accepted (version 12; owner, 2026-11-13)
task: issue #5 array receivers, with owner-directed array/slice pattern extensions
---

# Read literal arrays directly and match sequence shapes

## Rule

A literal array may be a method receiver as well as a call argument or
const initializer. Parentheses, token groups, and shared references keep
the expression in that position. For example, `[a, b].concat()`,
`["a", "b"].map(String::from).to_vec()`, and `(&[a, b]).len()` are allowed.
Each element and method still obeys its existing rules.

Array and slice patterns are allowed wherever Rust accepts their shape.
An exact pattern is `[first, second]`. A variable-length pattern may have
one rest, as in `[first, ..]`, `[first, remaining @ ..]`, or
`[first, middle @ .., last]`. A rest belongs directly to a slice element,
optionally under immutable `@` bindings; this does not admit tuple rests
or tuple-struct rests such as `Some(..)`.

Matches must cover every possible length and element value. Use an
explicit fallback or an exhaustive partition such as `[]` and `[_, ..]`.
Rust checks exhaustiveness and rejects refutable `let` patterns. A fixed
byte array can be destructured with `let [first, second] = bytes;`.

## Before and after

Conductor #5 reports three rounds of array receiver friction, including
`[a, b].concat()` and `["x", ..].map(String::from).to_vec()`. At `dac5e9c`,
`crates/daemon/tests/e2e.rs:1489` and `:2640` each allocate an intermediate
`Vec` before concatenating. The first becomes:

```rust
let view = fold([first, second, asked, ended].concat());
```

Slice patterns are an explicit extension requested by the owner, beyond
the report. Instead of `split_first()` followed by a tuple pattern:

```rust
fn payload(bytes: &[u8]) -> Option<Vec<u8>> {
    match bytes {
        [0x01, remaining @ ..] => Some(remaining.to_vec()),
        _ => None,
    }
}
```

## Weight and evidence

Arrays remain **neutral, 1.0**. Slice and slice-rest patterns are
**encouraged, 0.5**: they expose structure and length in the branch.
Boundary scoring keeps the existing rules; pattern syntax gains no new
boundary exception.

The two exact Conductor receiver sites above remove their intermediate
`Vec::from` calls. The report counts additional sites without retaining
all round logs. No new full-project Conductor measurement is claimed.
Representative isolated before/after examples, including the
owner-directed slice extension, are retained in
[0047-0048-syntax](0047-0048-syntax/measurements.json). They are examples,
not a measurement of the Conductor files. Reproduce with
`python3 docs/proposals/0047-0048-syntax/measure.py` after `cargo build`.

| Isolated example | Before score / tokens / lines | After score / tokens / lines |
|---|---|---|
| `joined` and `mapped` | 57.0 / 58 / 6 | 45.5 / 45 / 6 |
| `payload` | 40.0 / 44 / 6 | 34.5 / 42 / 6 |

The receiver examples save 20.2% in score; the slice example saves 13.8%.
These examples demonstrate the forms and use version 12's actual tiers.

## Qualities

1. **Named states: kept.** Domain models stay named. Tables with two or
   more rows and more than two tuple fields still need a row struct;
   grouping does not hide fields.
2. **Visible branching: kept.** Length and element alternatives appear
   in exhaustive arms; guards require an unguarded covering fallback.
3. **Forward data flow: kept.** Literal receivers introduce no bindings
   or mutable references. Rest bindings are immutable.
4. **Errors are values: kept.** Short or malformed sequences have an
   explicit arm; indexing is still outside the subset.
5. **Local reasoning: kept.** The pattern shows the sequence's shape.
6. **One way: kept.** Lists use array literals; sequence shape is matched.
7. **Repetition is data: kept.** Arrays remain data and multi-field rows
   remain named structs.

## Enforcement and limits

The walker propagates expression positions through grouping and shared
references, and registers only legitimate slice rests. Guard fallback
coverage compares minimum lengths and constrained prefixes/suffixes.
As with 0040, one later arm must cover the guarded pattern: coverage split
across several arms can still be rejected. For flexible rests, constraints
that are implied only by overlapping opposite ends are conservatively
rejected; a broader fallback expresses the case directly.

Rust supplies type checking, borrow checking, and exhaustiveness.
Regression tests compile the positive receiver, `@`, and slice examples;
three missing length/guard fallbacks fail with E0004, and moving owned
fields while binding the whole tuple fails with E0382.

At version 12, array types retained 0015's byte-only rule and slice
outputs retained 0014's positions. Version 13 adds ranges under
[0049](0049-pat.range.md). Version 14 adds immutable array locals, fields
and returns ([0050](0050-immutable-domain-arrays.md)), shared slice outputs
([0051](0051-shared-slice-outputs.md)), and reference/tuple rests
([0053](0053-reference-and-rest-patterns.md)). Mutable literal borrowing,
indexing and nonliteral repeat elements remain outside.

## Owner decision

The owner approved literal array receivers and array/slice patterns with
explicit fallbacks where lengths vary on 2026-11-13. Implemented in
version 12 alongside 0048.
