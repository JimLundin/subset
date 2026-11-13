# Where the subset gives ground

The thesis is that **every state is a named enum variant and every decision is
a `match`**, protected by seven qualities (`docs/subset.md`). Version 14 adds
the owner-directed patterns and data models in 0050–0057. Most proposals keep all
the qualities. The ones below don't, or they bend
the thesis at its edges. Each is confined by its rule, and this page is the
list to revisit first when one of them starts to spread.

## Qualities weakened

| Proposal | What gives way | Why it was accepted | What confines it |
|---|---|---|---|
| **0008** `mut` and `&mut self` | **Forward data flow.** An actor's methods change `self` instead of returning a new value. | Without it, the actor rebuilds its whole state on every event. That was 29% more code, and O(n²) as the event log grows. | Only an actor's loop state, a value that loop state owns (version 4: a record or buffer held in one of its fields, changed through that value's own `&mut self` methods), and external APIs that take `&mut`. No `&mut` parameter of our own types, and no `+=` anywhere. Every other function builds values. |
| **0035** `&mut self` on stream endpoints | **Forward data flow**, for the listed stream types. Their methods change the endpoint in place, anywhere in the code, not only inside loop state. | A stream's state (bytes consumed, the peer's window) lives outside the program, so returning the half by value only made every caller thread the handle back. The client's stream code shrank 21.9% (score -19.5%), and `FrameBuffer` 18.6%. | A named list, `endpoints` in `subset.toml`, that grows only by review. Each type wraps a library stream or channel whose own API takes `&mut`, or is the reassembly buffer of one. No `&mut` parameters of our types. Every other type of ours keeps 0008's reach. |
| **0018** Dioxus (`rsx!`, signals) | **Visible branching** and **forward data flow**, inside the UI crate. Markup can branch where the checker can't see, and signals are reactive state that Dioxus owns. | There is no other way to build the UI. | `crates/app` only. Decisions are made in `client`, and markup only renders their results. |
| **0009** trait impls | **Local reasoning**, slightly. A library calls our code from somewhere else. | There's no way around it for `Drop`, hand-written `Debug`/serde, and framework hooks such as iroh's `ProtocolHandler`. | A closed list of traits. `Display`, `From` and `FromStr` are written as plain methods instead. |
| **0043** `bool` on wire types | **Named states**, at the wire: a protocol's flag is a `bool` field. | The vendored SDKs' JSON has `is_error` and `interrupt` flags; without it each needed a duplicate struct or a bool-carrying enum (issue #1). | Only fields of serde-derived types, which 0012 confines to the wire modules. A flag read from the wire becomes an enum before it reaches our types, by review. |
| **0055** field defaults | **Visible branching**, at the wire: absence selects a default inside a derive. | Protocols omit fields that have a defined default; the owner approved this boundary form in #6. | Plain field metadata on serde-derived wire types. Review checks that absence means the default and preserves meaningful missing states. |
| **0045** `Visit` | **Forward data flow** and **local reasoning**, inside a visitor: syn calls our `&mut self` methods as it walks, and state lives on the visitor. | The checker's walk is 27% smaller in tokens than one fn per syntax node kind (proposal 0045). | Only implementors of syn's `Visit`, which count as 0035 endpoints and nothing more. |
| **0011** `#[from]` | **Local reasoning**, slightly: the error conversion isn't written at the `?`. **One way** is at risk, since `?` alone and `map_err` both exist. | It is 17% shorter across 20 sites. | Only when exactly one variant holds that error type, ours or a library's. Otherwise use `map_err`, so a fact about the enum decides between the two, not taste. |

## The thesis bends at its edges

- **Library shapes come back at the boundary.** 0023 allows `()`, `bool`, `true`/`false` and library `&mut` where an API we don't define demands them: in a type argument, call argument or field value of something we don't define, as in `EventHandler<()>`, `watch::channel(false)` and `tx.send(())`. 0019 also allows a `bool` for a command-line flag. Neither reaches a field, variant payload, parameter or return type of our own items, but the rule "no `bool`" is now "no `bool` past the edge".
- **Branching outside `match`.** 0016 `cfg` is a compile-time branch. It is kept visible by being item-level and paired. 0006 `.await` and 0007 `loop` add suspension and repetition outside iterator adapters, both needed for actors and I/O.
- **A second branching form.** 0027 allows `if let` for "act only on this case", because clippy's `single_match` rejects the `match` with an empty arm and no lint is switched off (0025 revised). It is confined to one branch: with an `else`, an `else if` or a let-chain it is a `match` again, so every two-way decision is still a `match`.
- **`bool` comes back inside predicates.** 0028 and 0037 let a closure passed straight to a method in `predicate_adapters` (`filter`, `find`, `rfind`, `any`, `all`, `position`, `take_while`, `skip_while`, `retain` and tokio's `wait_for`) return a boolean expression: `matches!`, a comparison, `&&`, `||` or `!`. This is the one place a decision is written as a `bool` rather than a `match`. It is confined to the closure body, the adapter is the branch, and the `bool` never becomes a binding, field, parameter or return type of ours. `true` and `false` stay out of it.
- **A guard on a match arm.** 0040 lets an arm carry a guard of one comparison or one `matches!`, because a pattern can't name a runtime value and equality of two runtime values had no `match` form. The guard is the 0028 boolean moved onto the arm it selects. It holds no `&&`, `||` or `!`, and a later arm with no guard must cover the same pattern, so the otherwise case is still a written-out arm. It replaces the `Some(x).filter(..)` and `a.eq(b).then_some(..)` lifts of one comparison. A lift that combines library booleans stays.
- **A lifetime outside library impls.** 0042 lets a `const` table's row struct have `&'static str` fields, when the table is scanned in order with one predicate, because the alternative was an enum with one `match` per column (-17.6% score on `device.rs`). **One way** is at risk: a scan and a keyed lookup look alike, and a keyed lookup stays a `match`. The checker flags a table searched for the row whose string field equals a value, and `&'static str` anywhere but in such a row struct.
- **Iteration outside adapters.** The actor `loop` (0007) and, since 0041, a `for` whose body only runs awaited calls with `?` are the two loops. A `for` produces no value: building or selecting values in one is a violation, and every other loop is still an adapter.

## Enforcement is partly by review

**Range patterns (0049, version 13).** Integer, byte, and character
intervals name cases directly, with immutable `@` bindings and the usual
exhaustiveness rules. Literal bounds are checked and ordered for guarded
fallback coverage; constant endpoint types remain review checks. Rust
permits float constant ranges, so Rust compilation alone does not enforce
the subset's integer/character restriction for an unresolved constant.
Identical symbolic ranges are recognized, while different constant names,
equivalent exclusive endpoints, and integers beyond `i128` require a
fallback the checker can prove structurally. Range expressions keep
their previous limits; 0054 subsequently adds scoped wire boolean patterns.

**Sequence and whole-value patterns (0047/0048, version 12).** Array
literals can be method receivers; immutable `@` bindings can wrap every
otherwise-permitted pattern, with grouping. Slice patterns expose length
and element shape without indexing. Rust checks exhaustiveness and
ownership: short lengths need fallback arms, and binding an owned tuple
while moving its non-`Copy` fields fails. Guarded sequence coverage checks
prefixes, suffixes and minimum lengths conservatively. Versions 13/14
add ranges, shared-reference patterns and tuple/tuple-struct rests. Only
slice rests may bind a name; Rust checks arity and borrowed-field moves.

**Immutable models (0050–0053, version 14).** Array cardinality carries
domain meaning; incidental constant lists stay slices. Shared outputs
borrow existing data under Rust lifetime elision. Unbounded generic
models preserve typed identities without new trait bounds or `where`.
The checker enforces syntax and direct mutable array restrictions;
meaningful cardinality, appropriate abstraction, aliases and indirect
inferred mutable uses remain review work.

**Boundary data (0054–0057, version 14).** Wire boolean patterns convert
flags to named states; location is checked, provenance and escape are
review work. Field `serde(default)` hides an absence decision in the
derive, so the chosen default must preserve protocol meaning. Tagged
unit `serde(other)` makes future events explicit but discards their
payload. `include_str!` supplies immutable build-time text; Rust checks
the file and rebuild dependencies. Typed `#[source]` preserves the cause
alongside error context; Rust/thiserror check trait obligations. No new
project-wide Conductor savings are claimed for these owner extensions.

**Local helpers (0046, version 11).** An immutable local closure keeps
its captures and uses in one block, with parameter annotations only when
needed. This adds no closure signatures or named boolean predicates.
A helper shared across functions stays a free function; the measurement
of two cross-test helpers grew when their bodies were copied into local
closures. The checker verifies reference positions and conservatively
rejects unknown macro references, while callee behavior and choosing the
shared-function form remain review checks.

**Synchronous receiver drains (0058, version 15).** The exact library
constructor, inline zero-argument `try_recv().ok()` pull and immediately
consuming inline `find_map` are checked. Named callbacks, constructor
aliases and escaping generators are rejected. Known own receiver bindings
and shadowed `std` modules are rejected where existing name facts identify
them. Binding origins follow lexical scopes; raw names and grouped imports
retain their logical identity. `::std` keeps the external boundary beside
a local shadow. Unresolved receiver fields and method semantics remain review work:
the pull must be a nonblocking external API under the existing mutation
rule. The closure keeps its tolerated tier and library boundary pricing.

`tools/subset` checks the location rules as `[[scope]]` tables in
`subset.toml`, the shape rules it can see in the syntax, and the interior
mutability types (`README.md`). It has no type information and
doesn't expand macros, so a reviewer still checks these:

- **Whose type it is.** The tool knows our types and functions only by name.
  0008(b) `let mut` for an external value is approximated as "never
  reassigned" and "not of a type of ours where the syntax names it": a
  declared parameter type, a struct literal, or an associated fn of a type
  of ours (assumed to return that type). A `mut` binding from a method
  call or a fn of ours, or one bound in a `match` arm, is not typed and
  passes. A cursor is the case to watch: `let mut rest = bytes;` passes as
  a library value lent as `&mut`, and only the 0039 list stops the reader
  that would reassign it. 0023's "something we don't define" is
  approximated as "not a name we define", with a method call's receiver ours only when it is `self`. A
  library wrapper hides where a value ends up: the tool allows
  `Option<bool>` as a field of our struct and `Some(true)` as its value,
  because `Option` and `Some` aren't ours. The reviewer checks that a
  boundary value doesn't flow on into our types. 0008's "a value that loop
  state owns" is approximated as "a type of ours named in a field type of a
  loop-state type", one step down, with enums of ours transparent so
  their variant payloads count too. 0024's "never compare through `.0`" is
  checked only where the receiver is `self` in an impl of an ordered
  newtype or a parameter declared as one. 0028's predicate leaves aren't
  typed, so `|x| x.flag` on a library `bool` passes.
- **Why a construct is there.** 0002 `pub` only for what another crate uses,
  0006 `async fn` only for I/O, 0012 serde only on types that cross a wire
  (the tool checks the location), 0013 codec bounds only at the wire boundary, 0052 generic models where
  types preserve meaningful distinctions (the tool checks parameter shape),
  0014/0051 slices for read-only sequences, 0015/0050 fixed arrays only
  where cardinality matters, 0009 generics
  only where the trait's signature demands them (the tool allows any inside
  a listed trait impl). Also 0024 an order that means something (the tool
  checks that the type is a newtype), 0025 `drop` only of a value that owns
  a resource released early (the tool checks that the argument is a plain
  name, and flags `drop` passed as a function), 0026 every round trip going
  through `ask` and its closure only building the variant (the tool checks
  the helper's shape; issue #3's one shared helper in a crate whose handles
  all use one stopped-error type remains a review check), 0036 a struct
  update's method-call base returning the struct's own type (the tool
  accepts any method call), and 0028 a `fold` or
  `map(..).sum()` used only to count (the tool flags
  `filter_map(..).count()`), and 0041's `stream::iter(..).then(..)` used
  only for effects (the tool flags `for_each`, `try_for_each` and a
  `try_collect` from `stream::iter` without a non-`()` list type; a
  `collect` into `Vec<Result<(), E>>` that is then checked for errors
  passes).
  0040's fallback arm is matched by pattern structure, so a fallback split
  across several arms is flagged even when it is exhaustive. 0040's guard
  can't be typed either: `Some(v) if v.is_empty() == w.is_empty()` compares
  two library booleans and passes, because `v` is bound. A lift whose
  result is bound first and matched later (`let due = opt.filter(..);`
  then `match due`) passes, and so does `match a.cmp(&b)` with only
  `Ordering::Equal` and a catch-all, which is an equality written as an
  ordering. 0041's `try_collect` needs the list's type in its turbofish to
  be told from effects, and a stream bound to a name is followed only
  within one fn.
- **What a table is.** 0042's row struct and keyed lookup are read from one file: a `const` of type `&[Row]` or `[Row; N]`, with `Row` resolved by its module path, whose literals stand only in such a table's array literal. The checker follows the forms around the `==` (`&&`, `||`, `!`, blocks, `any`, `enumerate`, destructured rows, `PartialEq::eq`, `cmp(..).is_eq()`, a `let` binding of the table, a table of bare `&str`). Left to review: a table passed to a helper fn, a `map` projection before the lookup, and near-equalities such as `eq_ignore_ascii_case`. The owner's ruling on 0001 (2026-11-12), clarified by the coordinator, makes a table of two or more rows whose rows have more than two fields, counted through nested tuples, a list of a named struct; it is checked on array literals, and a table built through a macro is left to review. A range built by name (`RangeInclusive::new`) is recognised by its std type name and allowed only where it is iterated at once or tested with `contains`; one bound to a name and iterated later is flagged, which is stricter than the rule needs. Apart from 0046's local-closure reference check, rules do not read macro arguments, so `assert_eq!(get(1..=2), 3)` passes.
- **Which trait a method belongs to.** 0038 matches a method call by
  name, and only in a file that imports a listed operator trait. A
  library prelude that re-exports one through a glob hides it, and in a
  file that imports `Add`, a method `add` of ours is flagged as well. 0039
  stops a consuming reader where the code names it (`use bytes::Buf`), not
  its methods, and only the readers on the list.
- **Macro bodies.** 0007's "each `select!` arm is a single call", Dioxus
  markup (0018) and `tracing` fields are not parsed.
- **What a body does.** 0017's callback only forwards the event to a
  channel, 0018's logic lives in `client`, and 0019's `main` turns the flag
  into an enum before passing it on.
- **Pairs by name.** 0016 counts a `cfg` fn or impl as paired when the other
  platform's item has the same name, and any `use` pairs with any `use`. A
  `cfg` module needs no pair, since the rule exempts modules that exist on
  one platform only.
