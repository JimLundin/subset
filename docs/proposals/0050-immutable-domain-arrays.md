---
construct: type.array, expr.array
status: accepted (version 14; owner, 2026-11-13)
task: issue #6, immutable domain data beyond Conductor's examples
---

# Put meaningful cardinality in the model

## Rule and syntax

An immutable fixed-size array may contain any otherwise-permitted type,
and may be a local, field, parameter or return value. Its length is an
integer literal or constant path, checked as `usize` by Rust:

```rust
struct RetrySchedule { delays: [std::time::Duration; 4] }
enum Provider { Claude, Codex }
const COUNT: usize = 2;
fn providers() -> [Provider; COUNT] {
    let providers = [Provider::Claude, Provider::Codex];
    providers
}
fn labels(first: String, second: String) -> [String; 2] { [first, second] }
```

Use an array where cardinality carries domain meaning. An ordinary
constant list remains `const PROVIDERS: &[Provider] = &[..]` under 0014;
this does not restore fixed arrays for lists with incidental lengths.
Access uses patterns, iteration or checked library methods, without indexing.
Existing literal-only repeat elements and named table-row rules remain.
No new mutation is admitted. Direct literals initializing `let mut` or
borrowed with `&mut` are rejected. Non-byte mutable array types are rejected
except required library trait signatures; existing byte-buffer rules remain.

## Evidence and weight

This is an explicit owner extension approved after Astra's #6 triage,
rather than a measured Conductor savings claim. The previous byte-only
rule required `Vec<Duration>` for a four-step schedule; `[Duration; 4]`
now makes the invariant a compiler check. `domain-data` compiles owned
non-`Copy` elements, constants, locals, returns and fields. Compiler
negatives prove wrong cardinality fails with E0308 and reuse after moving
owned elements fails with E0382. Checker negatives retain indexing and
mutable literal/type rejection.

`type.array` and `expr.array` retain **neutral, 1.0**. Existing element
type scoring and library boundary scoring continue to apply.

## Qualities and enforcement limits

Named states and cardinality are stronger; values flow forward without
mutation. Pattern matching keeps decisions visible and failures explicit.
Array literals provide one construction form; named rows preserve the
repetition-as-data rule. Length syntax is checked, but choosing meaningful
cardinality is review work. Indirect/inferred mutable array uses retain
0008's syntax-only type-resolution limit. This adds no const generics,
computed length expressions or indexing.

## Owner decision

The owner approved all remaining #6 additions on 2026-11-13. Version 14
supersedes the byte-only array restriction of 0015 and 0047.
