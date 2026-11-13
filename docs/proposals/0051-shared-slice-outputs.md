---
construct: type.slice
status: accepted (version 14; owner, 2026-11-13)
task: issue #6, borrowed outputs
---

# Return a shared view of existing data

## Rule and syntax

Shared slice return types are allowed, including within `Option` and
`Result`. Use Rust's lifetime elision to tie the view to an input or `self`:

```rust
fn payload(bytes: &[u8]) -> Option<&[u8]> {
    match bytes { [0x01, remaining @ ..] => Some(remaining), _ => None }
}
struct Packet { bytes: Vec<u8> }
impl Packet {
    fn bytes(&self) -> &[u8] { self.bytes.as_slice() }
}
```

The previous `Option<Vec<u8>>` payload needed `remaining.to_vec()`;
the new form returns the shared view without allocating or cloning.
Stored slice fields, mutable slice outputs and explicit domain lifetimes
are not added. Parameters and constant lists retain 0014's rules.

## Evidence and weight

The owner approved this extension after Astra ranked it in #6. No project
savings are claimed. The `domain-data` fixture compiles input- and
`self`-borrowed outputs and generic views. Compiler negatives demonstrate
that a view of a local buffer fails with E0515 and an ambiguous choice
between two input borrows fails with E0106. Checker negatives cover stored
and mutable slices. `type.slice` retains **neutral, 1.0**, including its
existing boundary scoring.

## Qualities and enforcement limits

The view retains element types and ownership, keeps selection in written
arms, and expresses absence as `Option`. Shared borrows preserve forward
data flow. Returning a view is the read-only sequence idiom; owned output
remains appropriate when ownership must move. Rust checks lifetimes and
aliasing. The walker checks that a returned slice has a reference and no
mutable reference ancestor, including nested return type arguments.
It does not resolve aliases or prove semantic provenance.

## Owner decision

Accepted in version 14 on 2026-11-13, extending 0014's return positions.
