---
construct: attr.source
status: accepted (version 14; owner, 2026-11-13)
task: issue #6 and Conductor #5 error-source friction
---

# Keep a typed cause alongside error context

## Rule and syntax

Plain `#[source]` is allowed on a field of a thiserror-derived error,
with at most one causal field per error struct or enum variant:

```rust
#[derive(Debug, thiserror::Error)]
enum LoadError {
    #[error("loading {path}")]
    Io { path: String, #[source] cause: std::io::Error },
}
```

The context names the failed operation, while the original error retains
its type and source chain. Construct the variant explicitly where context
is available. This does not add general `From` impls, custom source
expressions or sources on non-error types. Existing `#[from]` conversion
rules remain separate. Named `source` fields and `#[from]` fields count
when checking an explicit source's uniqueness.

## Evidence and weight

The runtime fixture verifies both the context message and the retained
source message. Checker negatives reject sources on non-errors, arguments
to `source`, two explicit sources, and an explicit source beside `#[from]`.
Rust/thiserror require the cause to implement the error trait and validate
the generated source implementation. This is an owner-approved candidate
without a measured Conductor savings claim. `attr.source` is **neutral,
1.0**, with existing boundary scoring.

## Qualities and enforcement limits

Errors stay typed values instead of flattened text. Context and cause
appear in one named variant, built without mutation; control flow remains
explicit at the construction site. One causal chain supports local
reasoning. The checker verifies the attribute host, plain metadata and
visible causal fields. Rust checks trait obligations and derive semantics;
aliases and information-preservation choices remain review work.

## Owner decision

Accepted in version 14 on 2026-11-13, extending 0010's error fields.
