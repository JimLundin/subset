---
construct: pat.reference, pat.rest
status: accepted (version 14; owner, 2026-11-13)
task: issue #6, structural patterns
---

# Select borrowed structure and elide unused fields

## Rule and syntax

Shared reference patterns `&Pattern` are permitted. An unbound `..` is
also allowed directly inside tuple and tuple-struct patterns:

```rust
fn borrowed(value: &Option<u8>) -> Option<u8> {
    match value { &Some(code) => Some(code), &None => None }
}
fn first(value: (u8, u8, u8)) -> u8 {
    let (first, ..) = value;
    first
}
fn keep(value: Option<String>) -> Option<String> {
    match value { whole @ Some(..) => whole, None => None }
}
```

Other forms include `(.., last)` and `(first, .., last)`. Immutable `@`
and grouping preserve inner rules. Only slice rests may bind a name,
as in `[first, remaining @ ..]`. `&mut` patterns, `ref`/`ref mut` bindings,
bound tuple rests and expression dereferencing remain outside.

Guarded patterns still require one later unguarded covering arm. Tuple
and tuple-struct rests reuse sequence prefix/suffix coverage; reference
patterns recurse into their contents. A fallback such as `(0..=4, ..)`
can cover guarded `(n @ 1..=3, _, _)`.

## Evidence and weight

The owner approved these forms beyond Conductor's dereference friction.
No new dereference expression rule is inferred. `structural-pattern`
compiles shared references, non-`Copy` borrowed records with copied
fields, tuple rests, whole-value tuple-struct rests and covering guards.
Moving an owned field through a shared pattern fails with E0507.
Negative checker cases retain mutable/reference binding and narrowed
fallback rejection. `pat.reference` is **neutral, 1.0**; `pat.rest` remains
**encouraged, 0.5**. Existing boundary scoring applies.

## Qualities and enforcement limits

Named models survive destructuring; selected and discarded structure
appears locally. Shared borrows preserve forward data flow and Rust
rejects moves from borrowed owners. Written fallbacks keep errors and
branches explicit. Rests elide unused structure without reconstructing it.
Guard coverage remains conservative where rests make positions uncertain,
or coverage is split across arms. Rust supplies arity, ownership,
refutability and exhaustiveness checks; aliases remain unresolved.

## Owner decision

Accepted in version 14 on 2026-11-13, extending 0047/0048's pattern set.
