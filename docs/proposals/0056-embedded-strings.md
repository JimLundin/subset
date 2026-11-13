---
construct: macro.allowed (include_str)
status: accepted (version 14; owner, 2026-11-13)
task: issue #6 and Conductor #5 embedded configuration
---

# Embed a checked-in text input at build time

## Rule and syntax

`include_str!` is permitted with one literal path in a const initializer:

```rust
const PINS: &str = include_str!("pins.toml");
```

Rust resolves the path relative to the source file and embeds UTF-8 text.
The constant can then be passed to an existing typed parser. Inline runtime
uses, computed paths, `concat!` arguments, `include!` and `include_bytes!`
are not added. Grouping preserves a direct initializer's position.

## Evidence and weight

The fixture embeds a real pins file and checks its contents. The CLI test
then changes that input, rebuilds, and verifies the new embedded text.
Negative cases reject a function-body use and a computed macro argument.
The owner approved this existing Conductor candidate as part of #6;
there is no new savings claim. `macro.allowed` retains **neutral, 1.0**,
with the existing boundary scoring policy.

## Qualities and enforcement limits

One immutable constant names the input, with no runtime filesystem state
or mutation. Typed parsing still expresses errors as values; branching
does not move into the macro. Repeated data can live in its source format.
The checker checks argument and initializer shape. Rust checks existence,
UTF-8 and rebuild dependencies. Review checks that the file belongs to
the project and parsing produces the intended model; the checker does
not read its contents or expand macros.

## Owner decision

Accepted in version 14 on 2026-11-13.
