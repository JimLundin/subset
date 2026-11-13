---
construct: generics.type_param
status: accepted (version 14; owner, 2026-11-13)
task: issue #6 and issue #1's typed Step identities
---

# Carry typed identities through one model

## Rule and syntax

Unbounded type parameters without defaults or `where` clauses are allowed
on structs, enums, inherent impls and functions:

```rust
struct Step<Id> { provider: Id, request: String }
enum Decision<Id> { Approved(Step<Id>), Rejected(Id) }
impl<Id> Step<Id> {
    fn new(provider: Id, request: String) -> Self { Self { provider, request } }
    fn with_request(self, request: String) -> Self { Self { request, ..self } }
}
fn keep<Id>(step: Step<Id>) -> Step<Id> { step }
```

Instead of converting provider ids to text newtypes to fit a nongeneric
`Step`, callers preserve their distinct types. Functions and impls can
move, borrow and package those types without inventing new trait bounds.
Multiple unbounded type parameters follow the same rule. Generic aliases,
user-defined traits, bounds, defaults, `where`, const generics and explicit
domain lifetimes are not added. Codec bounds and required library trait
signatures retain 0013/0009's exceptions; request helpers retain 0026's
separate shape rules.

## Evidence and weight

This implements the owner's approved modelling direction and the typed
identity example recorded in #1/#6. It does not claim new daemon savings
without a daemon rewrite. `domain-data` compiles generic structs, enums,
inherent impls, functions and borrowed views. Passing a different provider
identity fails with E0308. Negative fixtures reject bounds, defaults,
`where`, const parameters and lifetimes. The existing codec and helper
fixtures remain in the suite.

`generics.type_param` remains **tolerated, 2.0**. No scoring discount is
introduced for generic models; existing library signature scoring applies.

## Qualities and enforcement limits

Typed identities strengthen named states. Generic data moves forward,
and enum alternatives keep branching and errors visible. The unbounded
signature needs no trait search to understand its operations. One shared
model represents repetition without duplication. The walker checks each
parameter's context, bounds, default and `where` syntax. Appropriate
abstraction and identity meaning remain review work; Rust supplies type
and ownership checking. Derived library bounds remain Rust's responsibility.

## Owner decision

Accepted in version 14 on 2026-11-13, extending 0013 beyond codecs.
