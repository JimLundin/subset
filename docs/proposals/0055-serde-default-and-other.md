---
construct: attr.serde
status: accepted (version 14; owner, 2026-11-13)
task: issue #6 and Conductor #5 boundary data
---

# Model absent fields and future tagged events

## Rule and syntax

Plain `#[serde(default)]` is allowed on fields of serde-derived wire types.
Plain `#[serde(other)]` is allowed on a unit variant of a tagged,
serde-derived wire enum, including internally and adjacently tagged enums:

```rust
use serde::Deserialize;

#[derive(Deserialize)]
struct ModelRow {
    #[serde(default)]
    hidden: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    effort: Option<String>,
}
#[derive(Deserialize)]
#[serde(tag = "kind")]
enum Event { Open, #[serde(other)] Other }
```

The existing wire module scope still applies. Container defaults, custom
default functions, payload-bearing `other` variants and untagged/external
fallbacks are not added. Unknown tags become an explicit state; `other`
does not promise to retain their payload. The existing skip predicate is
still only `Option::is_none`. Serde checks variant order and representation.

## Evidence and weight

Runtime tests deserialize missing fields, distinguish a missing bool
from an invalid null bool, deserialize a null Option, verify omission
when serializing, and map unknown internal/adjacent tags to `Other`.
Negative fixtures exercise each misplaced or broader form. These are
owner-approved boundary extensions without a measured Conductor rewrite.
`attr.serde` remains **neutral, 1.0** and preserves boundary scoring.

## Qualities and enforcement limits

Future tags become named states. Defaulting is a wire-level compromise:
absence is handled by Serde rather than a visible match, so reviewers
must check that absence means the chosen default. It must not erase a
domain distinction that needs `Option` or a named variant. Values are
constructed immutably and errors remain deserialization errors. The
checker checks derive/field/variant hosts and tag metadata; Rust/Serde
check types and macro semantics. Aliases and protocol meaning are review work.

## Owner decision

Accepted in version 14 on 2026-11-13, extending 0012/0043's Serde keys.
