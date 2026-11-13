---
construct: pat.lit.bool
status: accepted (version 14; owner, 2026-11-13)
task: issue #6 and Conductor #5 boundary boolean friction
---

# Convert protocol flags into domain states

## Rule and syntax

Boolean literal patterns are allowed only in `crates/proto/**` and
`crates/agents/src/*/protocol.rs`, to convert boundary flags to named states:

```rust
enum Visibility { Visible, Hidden }
fn visibility(hidden: Option<bool>) -> Visibility {
    match hidden {
        Some(true) => Visibility::Hidden,
        Some(false) | None => Visibility::Visible,
    }
}
```

Plain `true`/`false`, tuple, struct, reference and `@` nesting follow the
same path scope. The boundary determines how absence maps to a state;
it may need its own variant instead of sharing the false arm. Ordinary
domain boolean signatures, values, predicates and compound guards retain
their existing limits. This does not admit Conductor's entire reported
`(1.., true, ..)` expression in an arbitrary domain module.

## Evidence and weight

The scoped fixture checks `Option<bool>` conversion and tests all three
inputs against named results. An outside-wire module rejects boolean
patterns. Existing 0043 bool-field and 0023 library-boundary tests remain.
This is an owner-approved extension, with no new Conductor savings claim.
`pat.lit.bool` is **encouraged, 0.5**, only inside the permitted scope;
existing boundary scoring remains.

## Qualities and enforcement limits

Named states are restored at the boundary. All flag cases appear in
match arms, and conversion introduces no mutation or hidden control flow.
Optional flags retain explicit absence handling. The syntax checker
enforces location, but cannot prove flag provenance or that a boolean
does not escape the module. Review must check the conversion's domain
meaning and output. Raw `bool` parameters remain rejected; the existing
library type-argument allowance admits `Option<bool>`.

## Owner decision

Accepted in version 14 on 2026-11-13, extending 0043 with scoped patterns.
