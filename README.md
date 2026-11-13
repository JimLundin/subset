# subset

`subset` enforces and measures the Conductor Rust subset described in
`docs/subset.md`. It started inside
[Conductor](https://github.com/JimLundin/conductor) and was extracted here
with its history; Conductor installs it at a pinned revision.

The tool is written in the subset it checks: `subset.toml` here is
Conductor's allowlist with this repository's `src/` and `tests/*.rs` added to
`include`, and a test keeps `subset check` clean on them.

To report friction or propose a construct from such a project, see
[Presenting a case](docs/presenting-a-case.md).

## Usage

Install it with `cargo install --git https://github.com/JimLundin/subset --rev <sha>`,
and run it from the root of the project it checks.

```sh
subset check [--config subset.toml]
subset report [--config subset.toml] [--json]
subset measure <paths...> [--json]
subset score [paths...] [--config subset.toml] [--json] [--by crate|file|fn|construct]
subset report-html --out <dir> [--config subset.toml] [--compare <dir>]... [--history <n>]
subset ids
subset score-ids
subset rules
```

- `check` prints one line per construct that isn't allowed in the files
  under `include`, as `path:line:col: <construct-id>: <reason>`. It exits
  with 0 when they are clean, 1 when there are violations, and 2 on a
  config or parse error.
- `report` groups the same violations by id, most frequent first, with the
  number of files and up to five locations. `--json` prints `{"total",
  "ids": [{"id", "reason", "count", "files", "examples"}]}`.
- `measure` counts the tokens and code lines of files, or of every `*.rs`
  file under a directory. Proposals compare the before and after of the
  same code with it.
- `score` prints the weighted score of `paths`, or of the files under
  `include`, per crate (the default), file, fn or construct: the score,
  tokens, lines, score per token, and the boundary score and its share.
  `--json` prints `{"by", "rows": [..], "total"}`.
- `report-html` writes `<dir>/index.html`, a static page of the score.
- `ids`, `rules` and `score-ids` print the tables below. A test keeps this
  file equal to their output.

## Configuration

`subset.toml` lists the ids code may use (`allowed`), the files it applies
to (`include`, globs relative to the config), the derives and macros
`attr.derive` and `macro.allowed` admit, and the extra `test_macros` of
test code. A `[[scope]]` allows ids only under its `paths`; a use elsewhere
is reported with its `rule` as the reason:

```toml
[[scope]]
rule = "Dioxus only in crates/app (proposal 0018)"
paths = ["crates/app/**"]
allowed = ["macro.rsx", "macro.asset", "attr.component"]
```

These lists tune the shape rules; a list that is given replaces its default:

- `endpoints`: types of ours whose methods may take `&mut self` anywhere
  (0035). Default none.
- `predicate_adapters`: methods whose closure may have a boolean body
  (0028, 0037). Default `filter`, `find`, `any`, `all`, `position`,
  `take_while`, `skip_while`, `retain`.
- `closure_methods`: the other methods a closure may be passed to. Default
  the std `Iterator`, `Option` and `Result` methods that take one.
- `fallible_operators`: operator traits that can overflow or panic, with
  their methods, never named or called by name (0038). Default the
  `std::ops` arithmetic and shift traits.
- `consuming_readers`: types that read by changing their buffer, never
  named (0039). Default `bytes::Buf`.

## Shape rules

An accepted proposal often allows an id only in some form or position.
`check` reports a use that breaks such a rule with the id and the rule. A
rule applies only where its id is allowed; elsewhere the id itself is the
violation. Rules that need other files see names gathered from every file
under `include`: our crates, modules, types and free fns, error types,
actor loop states and what they own, ordered newtypes, serde wire types and
`Visit` implementors. They are names, not resolved paths.

| Id | Rule |
|---|---|
| `vis.restricted` | only `pub(crate)`, never `pub(super)` or `pub(in …)` (proposal 0003) |
| `vis.pub` | a field of a `pub(crate)` struct is `pub(crate)`, never `pub` (proposal 0003) |
| `expr.struct.update` | the base of `..base` is a path, a constructor call or a method call (proposals 0004, 0036) |
| `type.unit` | `()` as a type only in a type argument of a type we don't define, as in `Result<(), E>`, or as `fn main`'s return type (proposals 0005, 0023) |
| `expr.unit` | `()` as a value only as an argument or field value of a fn, method or struct we don't define, as in `Ok(())` (proposals 0005, 0023) |
| `expr.async` | an async block only as a spawn argument or the whole body of an `unfold`, `then` or `use_future` closure (proposal 0006) |
| `attr.tokio::main` | `#[tokio::main]` only on `fn main` (proposal 0006) |
| `expr.loop` | loop must be the whole body of an actor run fn, after at most one `let mut` state (proposal 0007) |
| `expr.loop` | a loop turn takes one input (`select!`, a `match` of `select!`s, or one awaited call) and handles it with one call or `match` (proposal 0007) |
| `expr.break` | `break` only with a value, out of an actor loop (proposal 0007) |
| `item.fn.receiver.mut` | `mut self` only on a fn whose body is an actor loop (proposal 0008) |
| `item.fn.receiver.mut` | `&mut self` only on methods of an actor's loop state, of a type of ours named in one of its field types or in a variant of an enum it owns (proposal 0008), of a type in `endpoints` (proposal 0035), or of a type that implements syn's `Visit` (proposal 0045) |
| `stmt.let.mut` | `let mut` only for a loop's state just before the loop, or for a value never reassigned and only lent as `&mut` (proposal 0008) |
| `pat.ident.mut` | a `mut` binding is never reassigned, only lent as `&mut` (proposal 0008) |
| `stmt.let.mut` | `mut` only on a value whose type isn't ours, on an actor's loop state, on a type in `endpoints` or on a `Visit` implementor (proposal 0045); to lend a library field of a value of ours as `&mut`, destructure the value and bind that field `mut` (proposals 0008, 0035) |
| `pat.ident.mut` | `mut` only on a value whose type isn't ours, on an actor's loop state, on a type in `endpoints` or on a `Visit` implementor (proposal 0045); to lend a library field of a value of ours as `&mut`, destructure the value and bind that field `mut` (proposals 0008, 0035) |
| `expr.assign` | assignment only to `self.field` in a `&mut self` method, or to the loop's state binding (proposal 0008) |
| `type.reference.mut` | no `&mut` of our own types other than `&mut self` (proposals 0008, 0023) |
| `expr.call` | `drop(..)` only of a named value that owns a resource released early; discard a value with `let _ = value;` (proposal 0025) |
| `expr.path` | `drop` passed as a function, as in `.map(drop)`, is a discard; write `let _ = value;` (proposal 0025) |
| `expr.method_call` | `.ok()` is not a discard; write `let _ = value;` (proposal 0025) |
| `item.use` | a trait in `fallible_operators` is never imported, named in a path or called as a method; write `checked_*` or `saturating_*` (proposal 0038) |
| `expr.path` | a trait in `fallible_operators` is never imported, named in a path or called as a method; write `checked_*` or `saturating_*` (proposal 0038) |
| `type.path` | a trait in `fallible_operators` is never imported, named in a path or called as a method; write `checked_*` or `saturating_*` (proposal 0038) |
| `expr.method_call` | a trait in `fallible_operators` is never imported, named in a path or called as a method; write `checked_*` or `saturating_*` (proposal 0038) |
| `item.use` | a reader in `consuming_readers` changes its buffer on every read, so a tuple of reads means something only by evaluation order; split the bytes with `split_at_checked` and match them, as in the bootstrap review's §A7 (proposal 0039) |
| `expr.path` | a reader in `consuming_readers` changes its buffer on every read, so a tuple of reads means something only by evaluation order; split the bytes with `split_at_checked` and match them, as in the bootstrap review's §A7 (proposal 0039) |
| `type.path` | a reader in `consuming_readers` changes its buffer on every read, so a tuple of reads means something only by evaluation order; split the bytes with `split_at_checked` and match them, as in the bootstrap review's §A7 (proposal 0039) |
| `macro.matches` | a boolean expression (`matches!`, a comparison, `&&`, `\|\|`, `!`) only as the body of a closure passed straight to a method in `predicate_adapters` (proposals 0028, 0037), or one comparison or `matches!` as a match guard (proposal 0040) |
| `expr.binary.eq` | a boolean expression (`matches!`, a comparison, `&&`, `\|\|`, `!`) only as the body of a closure passed straight to a method in `predicate_adapters` (proposals 0028, 0037), or one comparison or `matches!` as a match guard (proposal 0040) |
| `expr.binary.ne` | a boolean expression (`matches!`, a comparison, `&&`, `\|\|`, `!`) only as the body of a closure passed straight to a method in `predicate_adapters` (proposals 0028, 0037), or one comparison or `matches!` as a match guard (proposal 0040) |
| `expr.binary.lt` | a boolean expression (`matches!`, a comparison, `&&`, `\|\|`, `!`) only as the body of a closure passed straight to a method in `predicate_adapters` (proposals 0028, 0037), or one comparison or `matches!` as a match guard (proposal 0040) |
| `expr.binary.le` | a boolean expression (`matches!`, a comparison, `&&`, `\|\|`, `!`) only as the body of a closure passed straight to a method in `predicate_adapters` (proposals 0028, 0037), or one comparison or `matches!` as a match guard (proposal 0040) |
| `expr.binary.gt` | a boolean expression (`matches!`, a comparison, `&&`, `\|\|`, `!`) only as the body of a closure passed straight to a method in `predicate_adapters` (proposals 0028, 0037), or one comparison or `matches!` as a match guard (proposal 0040) |
| `expr.binary.ge` | a boolean expression (`matches!`, a comparison, `&&`, `\|\|`, `!`) only as the body of a closure passed straight to a method in `predicate_adapters` (proposals 0028, 0037), or one comparison or `matches!` as a match guard (proposal 0040) |
| `expr.binary.and` | a boolean expression (`matches!`, a comparison, `&&`, `\|\|`, `!`) only as the body of a closure passed straight to a method in `predicate_adapters` (proposals 0028, 0037), or one comparison or `matches!` as a match guard (proposal 0040) |
| `expr.binary.or` | a boolean expression (`matches!`, a comparison, `&&`, `\|\|`, `!`) only as the body of a closure passed straight to a method in `predicate_adapters` (proposals 0028, 0037), or one comparison or `matches!` as a match guard (proposal 0040) |
| `expr.unary.not` | a boolean expression (`matches!`, a comparison, `&&`, `\|\|`, `!`) only as the body of a closure passed straight to a method in `predicate_adapters` (proposals 0028, 0037), or one comparison or `matches!` as a match guard (proposal 0040) |
| `expr.method_call` | count matching items with `filter(\|x\| matches!(x, ..)).count()`, not `filter_map(..).count()` (proposal 0028) |
| `expr.match.guard` | a match guard is one comparison (`==`, `!=`, `<`, `<=`, `>`, `>=`) or one `matches!`, with no `&&`, `\|\|` or `!` (proposal 0040) |
| `expr.match.guard` | a guard tests a value its arm's pattern bound; a guard on nothing the pattern bound is an `if` (proposal 0040) |
| `expr.match.guard` | a guard that compares with a literal, a variant or a constant, or a `matches!` on a place, is a pattern: write it in the arm (proposal 0040, bootstrap review §A3, §A4) |
| `expr.match.guard` | a guarded arm needs a later arm with no guard whose pattern covers the guarded one, so the otherwise case is written out (proposal 0040) |
| `macro.matches` | `matches!` takes a place or a call and a pattern with no `if` and no `true` / `false`, so no boolean rides through it (proposals 0028, 0040) |
| `expr.method_call` | `Some(x).filter(..)` or `.find(..)` with one comparison is a match guard: `pat if a == b => ..` with an unguarded arm for the rest (proposal 0040) |
| `expr.method_call` | a `filter` of one comparison matched at once is a match guard: `Some(x) if a == b => ..` with an unguarded arm for the rest (proposal 0040) |
| `expr.method_call` | a comparison by name (`a.eq(&b)`, `a.lt(&b)`, `a.cmp(&b).is_eq()`, `PartialEq::eq`) is a `bool` outside a predicate; write a match guard `pat if a == b => ..` (proposal 0040) |
| `expr.for` | a `for` body holds only calls, as `f(item);`, `f(item)?;`, `x.m(item).await?;` or `let _ = f(item);`, each building nothing; build or select values with the adapters (`map`, `filter`, `fold`, `try_fold`, ..) (proposal 0041) |
| `expr.for` | a `for` iterates a list built before it or by adapters, with no block, assignment or branch in its iterator expression (proposal 0041) |
| `expr.method_call` | a loop whose body performs side effects is `for item in list { .. }`, not `for_each` or `try_for_each` (proposal 0041) |
| `expr.method_call` | awaited effects run in order as `for item in list { f(item).await?; }`, not `stream::iter(..).then(..).try_collect()` into `()` (proposal 0041) |
| `item.impl.trait` | trait impls only for `Drop`, `Debug`, `Serialize`, `Deserialize`, `ProtocolHandler` (proposal 0009) and syn's `Visit` (proposal 0045) |
| `generics.lifetime` | lifetime parameters only in a library trait impl (proposal 0009) |
| `type.lifetime` | lifetimes only in a library trait impl, where its signature demands them (proposal 0009) |
| `type.lifetime.static_str` | `&'static str` only as a field type of a struct that is the row type of a `const` table and is built nowhere else (proposal 0042) |
| `expr.method_call` | a predicate on a `const` table that compares a row's string for equality with a value is a keyed lookup; write a `match` on the value, as in `match os { "macos" => .., _ => .. }` (proposal 0042) |
| `expr.call` | a range built by name (`RangeInclusive::new`, any other associated fn of a range type, `Range { .. }`, `RangeFull`, a `Bound`) is a range expression (`expr.range`) under another name; it stands only as the iterator of a `for`, the receiver of an iterator method or of `contains`, or the argument of `zip` or `chain`, never as an index or a slice argument: split bytes with `split_at_checked` (bootstrap review §A7, rtc-ports and v8 reviews) |
| `expr.struct` | a range built by name (`RangeInclusive::new`, any other associated fn of a range type, `Range { .. }`, `RangeFull`, a `Bound`) is a range expression (`expr.range`) under another name; it stands only as the iterator of a `for`, the receiver of an iterator method or of `contains`, or the argument of `zip` or `chain`, never as an index or a slice argument: split bytes with `split_at_checked` (bootstrap review §A7, rtc-ports and v8 reviews) |
| `expr.path` | a range built by name (`RangeInclusive::new`, any other associated fn of a range type, `Range { .. }`, `RangeFull`, a `Bound`) is a range expression (`expr.range`) under another name; it stands only as the iterator of a `for`, the receiver of an iterator method or of `contains`, or the argument of `zip` or `chain`, never as an index or a slice argument: split bytes with `split_at_checked` (bootstrap review §A7, rtc-ports and v8 reviews) |
| `expr.method_call` | `split_at` panics past the end; split bytes with `split_at_checked` and match the `Option` (bootstrap review §A7) |
| `type.tuple` | a `const` table whose rows hold a `&str` has a named row struct with `&'static str` fields, not a tuple row (proposal 0042, v8 review) |
| `expr.tuple` | a table of two or more rows with more than two fields each, counted through nested tuples, is a list of a named struct, not of tuples (proposal 0001, owner ruling 2026-11-12) |
| `attr.derive` | `thiserror::Error` only on an enum or named struct that is an error type, named `…Error` or used as a `Result`'s error (proposal 0010) |
| `attr.derive` | `PartialOrd` and `Ord` only on a newtype struct whose order means something (proposal 0024) |
| `expr.method_call` | compare an ordered newtype as a whole, `a.cmp(&b)`, never through `.0` (proposal 0024) |
| `attr.error` | each error variant carries one `#[error("…")]` message, never `transparent` (proposal 0010) |
| `attr.from` | `#[from]` only on the one variant of an enum holding a given error type (proposal 0011) |
| `attr.serde` | `#[serde]` only with `transparent`, `tag`, `content`, `rename_all` and `rename` (proposal 0012), `untagged`, or `skip_serializing_if = "Option::is_none"` (proposal 0043) |
| `generics.type_param` | a type parameter only on a fn, with exactly one bound, `Serialize` or `DeserializeOwned`, and no `where` (proposal 0013), or as the unbounded `T` of an `ask` helper (proposal 0026) |
| `type.impl_trait.arg` | `impl Trait` as an argument only as `build: impl FnOnce(oneshot::Sender<T>) -> Msg`, the last parameter of a private `ask<T>` helper (proposal 0026) |
| `type.slice` | `&[T]` only as a parameter type or a `const` type; a `const` list is `const NAME: &[T] = &[..]` (proposal 0014) |
| `type.array` | an array type only for bytes whose size is part of a format or API (`[u8; N]`); a `const` list is `&[T]` (proposals 0014, 0015) |
| `expr.array` | an array literal only passed straight to a call, as in `f([a, b])` or `f(&[a, b])`, or as a `const` value (proposal 0015) |
| `expr.repeat` | an array repeat only of a literal, as in `[0u8; N]` (proposal 0015) |
| `attr.cfg` | `cfg` only with `target_family = "wasm"`, `unix` or their `not` (proposal 0016) |
| `attr.cfg` | `cfg` only on a mod, use, fn or impl item, or as `#![cfg]` atop a test file (proposal 0016) |
| `attr.cfg` | a `cfg` fn, impl or use needs its counterpart for the other platform in the same file (proposal 0016) |
| `type.dyn` | `dyn` only as `Closure<dyn FnMut(..)>` (proposal 0017) |
| `expr.closure.adapter` | a closure passed to a method only to a method in `closure_methods` or `predicate_adapters`: the iterator, `Option` and `Result` methods and the library methods the config lists (proposals 0006, 0026, 0028, 0037) |
| `expr.closure` | a closure outside a method call only as the argument of `Closure::new`, `unfold`, `ask` or a Dioxus hook (proposals 0006, 0017, 0018, 0026) |
| `item.use.glob` | a glob import only for `dioxus::prelude::*` (proposal 0018) |
| `attr.allow` | only `#![allow(clippy::useless_format)]` at the crate root (proposal 0018) |
| `type.bool` | `bool` only in a type argument of a type we don't define, as a flag field of a clap `Parser` struct, or as a field of a serde wire type (proposals 0019, 0023, 0043) |
| `attr.arg` | a choice between named values is a `ValueEnum`, not `value_parser = [...]` (proposal 0019) |
| `attr.tokio::test` | `#[tokio::test]` takes no arguments and marks an `async fn` (proposal 0020) |
| `pat.ident.at` | `name @ pattern` only binds a whole value matched by a struct, tuple-struct or path pattern, with no `mut` or `ref` (proposal 0044) |
| `expr.lit.bool` | `true` / `false` only as an argument or field value of a fn, method or struct we don't define, or as a field value of a serde wire type (proposals 0023, 0043) |
| `macro.env` | `env!` only for `CARGO_PKG_VERSION`, `CARGO_PKG_NAME` and `CARGO_MANIFEST_DIR` (proposal 0022) |

## How constructs are classified

Every item, expression, statement, pattern, type, generic parameter,
attribute and macro invocation gets one id; structural nodes carry none,
but their modifiers do (`async` on a fn is `item.fn.async`). A syn node kind
the tool doesn't know is `unknown.<kind>`, never allowed. A closure passed
straight to a method is `expr.closure.adapter`; an `if let` with no `else`
is `expr.if_let.single`; a `let` with a `mut` binding is `stmt.let.mut`; a
derive of a name outside `derives` is `attr.derive.<Name>`; a macro outside
`macros` is `macro.<path>`; std cells, locks and atomics are
`type.interior_mut.<Name>` or `expr.interior_mut.<Name>`.

## Measuring

Tokens are `proc_macro2` leaves: each identifier, punctuation character and
literal counts one, and each delimited group one for its pair. Lines are
those holding a counted token. Comments and doc comments don't count.

## The weighted score

Every token costs the weight of the tier of the innermost construct that
contains it: `encouraged`, `neutral`, `tolerated`, `costly`, `outside` or
`boundary`, each weighted in `[weights.tiers]`, with ids given tiers in
`[weights.ids]` (an allowed id missing there is neutral). A violation
costs `outside`. A construct whose shape an item we don't define fixes
costs `boundary`: a trait impl of a library's, a library attribute, derive
or macro, a `mut` binding lent to a library, and a tolerated or costly
construct that is an argument, field value or type argument of something we
don't define. The head of an `if let` and a primitive or untyped type fold
their insides into themselves.

The score refines a few ids by where they stand, so they take their own
tier:

| Id | Meaning |
|---|---|
| `expr.method_call.adapter` | method call whose name is in `adapters` |
| `stmt.let.discard` | `let _ = value;` |
| `type.field` | type in a field of our own struct or enum variant |
| `type.primitive` | type in `primitives`, as a field type or in a signature of our own |
| `type.untyped` | type in `untyped`, anywhere |
| `field.wire` | field of a wire type in an external protocol module |

`[weights]` also takes `adapters`, `primitives`, `untyped`, `transparent`
(std containers typing sees through, as `Option`) and `external_protocols`
(globs of modules mirroring a protocol someone else defines, whose wire
fields cost `boundary`).

## The HTML report

`report-html` writes one page with no script. Each group, the files under
`include` and each `--compare <dir>`, shows tiles (score, score in lines,
tokens, per token, boundary share, violations), the score per crate as
stacked tier bars, a treemap of files sized by score and coloured by score
per token, the typing of fields and signatures per crate, and constructs and
fns by score. `--history <n>` adds the score per crate at each of the last
`n` commits that changed files under `include`, read with git and scored
under the current config.

## Known limits

- Macro bodies aren't checked; the macro's id is.
- Nothing is typed: "ours" means a path from `crate`, `self`, `super`, our
  crates or modules, or a name we define, and a method's receiver is ours
  only when it is `self`. A library wrapper hides where a value goes, so
  `Option<bool>` as our field passes.
- Some rules stay with the reviewer, as `docs/compromises.md` lists: why a
  construct is there (0002, 0006, 0012, 0013, 0014, 0015) and what a body
  does (0017, 0019).

## Construct ids

| Id | Meaning |
|---|---|
| `item.struct.named` | struct with named fields |
| `item.struct.newtype` | struct with exactly one unnamed field |
| `item.struct.tuple` | tuple struct with zero or several unnamed fields |
| `item.struct.unit` | unit struct |
| `item.enum` | enum |
| `item.enum.discriminant` | explicit enum discriminant |
| `item.fn` | function or method |
| `item.fn.async` | async fn |
| `item.fn.const` | const fn |
| `item.fn.unsafe` | unsafe fn |
| `item.fn.extern` | fn with an explicit ABI |
| `item.fn.variadic` | variadic fn |
| `item.fn.receiver.mut` | `mut self` or `&mut self` receiver |
| `item.fn.receiver.typed` | self receiver with an explicit type |
| `item.impl.inherent` | inherent impl |
| `item.impl.trait` | trait impl |
| `item.impl.unsafe` | unsafe impl |
| `item.impl.type` | associated type in an impl |
| `item.const` | const item |
| `item.static` | static item |
| `item.static.mut` | static mut item |
| `item.use` | use declaration |
| `item.use.glob` | glob import |
| `item.use.rename` | import renamed with `as` |
| `item.mod` | module |
| `item.extern_crate` | extern crate |
| `item.foreign_mod` | extern block |
| `item.foreign.fn` | fn in an extern block |
| `item.foreign.static` | static in an extern block |
| `item.foreign.type` | type in an extern block |
| `item.trait` | trait definition |
| `item.trait.unsafe` | unsafe trait |
| `item.trait.auto` | auto trait |
| `item.trait.const` | associated const in a trait |
| `item.trait.fn` | fn in a trait |
| `item.trait.type` | associated type in a trait |
| `item.trait_alias` | trait alias |
| `item.type` | type alias |
| `item.union` | union |
| `item.macro_rules` | macro_rules! definition |
| `vis.pub` | `pub` visibility |
| `vis.restricted` | `pub(crate)`, `pub(super)` or `pub(in path)` visibility |
| `expr.array` | array expression |
| `expr.assign` | assignment |
| `expr.async` | async block |
| `expr.await` | `.await` |
| `expr.binary.add` | `+` operator |
| `expr.binary.sub` | `-` operator |
| `expr.binary.mul` | `*` operator |
| `expr.binary.div` | `/` operator |
| `expr.binary.rem` | `%` operator |
| `expr.binary.and` | `&&` operator |
| `expr.binary.or` | `\|\|` operator |
| `expr.binary.bit_xor` | `^` operator |
| `expr.binary.bit_and` | `&` operator |
| `expr.binary.bit_or` | `\|` operator |
| `expr.binary.shl` | `<<` operator |
| `expr.binary.shr` | `>>` operator |
| `expr.binary.eq` | `==` operator |
| `expr.binary.lt` | `<` operator |
| `expr.binary.le` | `<=` operator |
| `expr.binary.ne` | `!=` operator |
| `expr.binary.ge` | `>=` operator |
| `expr.binary.gt` | `>` operator |
| `expr.binary.add_assign` | `+=` operator |
| `expr.binary.sub_assign` | `-=` operator |
| `expr.binary.mul_assign` | `*=` operator |
| `expr.binary.div_assign` | `/=` operator |
| `expr.binary.rem_assign` | `%=` operator |
| `expr.binary.bit_xor_assign` | `^=` operator |
| `expr.binary.bit_and_assign` | `&=` operator |
| `expr.binary.bit_or_assign` | `\|=` operator |
| `expr.binary.shl_assign` | `<<=` operator |
| `expr.binary.shr_assign` | `>>=` operator |
| `expr.block` | block expression |
| `expr.block.labeled` | labeled block |
| `expr.break` | `break` |
| `expr.call` | function call |
| `expr.cast` | `as` cast |
| `expr.closure` | closure that is not a direct argument of a method call |
| `expr.closure.adapter` | closure passed straight to a method call |
| `expr.closure.async` | async closure |
| `expr.closure.const` | const closure |
| `expr.closure.static` | static closure |
| `expr.const_block` | const block |
| `expr.continue` | `continue` |
| `expr.field` | field access |
| `expr.for` | `for` loop |
| `expr.group` | invisible group expression |
| `expr.if` | `if` expression |
| `expr.if_let` | `if let` with an `else`, or an `if` with a let-chain |
| `expr.if_let.single` | `if let` with no `else` |
| `expr.index` | indexing |
| `expr.infer` | `_` in expression position |
| `expr.let` | `let` inside a condition chain |
| `expr.lit` | literal other than `true` / `false` |
| `expr.lit.bool` | `true` / `false` literal |
| `expr.loop` | `loop` |
| `expr.match` | `match` expression |
| `expr.match.guard` | `if` guard on a match arm |
| `expr.method_call` | method call |
| `expr.paren` | parenthesized expression |
| `expr.path` | path expression |
| `expr.path.qualified` | qualified path `<T as Trait>::item` |
| `expr.range` | range expression |
| `expr.raw_addr` | `&raw` pointer |
| `expr.reference` | `&x` |
| `expr.reference.mut` | `&mut x` |
| `expr.repeat` | array repeat `[x; n]` |
| `expr.return` | `return` |
| `expr.struct` | struct or enum-variant construction |
| `expr.struct.update` | struct construction with `..base` |
| `expr.try` | `?` operator |
| `expr.try_block` | `try` block |
| `expr.tuple` | tuple expression |
| `expr.unit` | `()` value |
| `expr.unary.deref` | `*` dereference |
| `expr.unary.not` | `!` operator |
| `expr.unary.neg` | unary `-` |
| `expr.unsafe` | unsafe block |
| `expr.while` | `while` loop |
| `expr.while_let` | `while let` loop |
| `expr.yield` | `yield` |
| `stmt.let` | immutable `let` with an initializer |
| `stmt.let.mut` | `let` with a `mut` binding |
| `stmt.let.else` | `let ... else` |
| `stmt.let.uninit` | `let` without an initializer |
| `pat.const` | const block pattern |
| `pat.ident` | binding pattern |
| `pat.ident.mut` | `mut` binding outside `let` |
| `pat.ident.ref` | `ref` binding |
| `pat.ident.ref_mut` | `ref mut` binding |
| `pat.ident.at` | `name @ pattern` binding |
| `pat.lit` | literal pattern other than `true` / `false` |
| `pat.lit.bool` | `true` / `false` pattern |
| `pat.or` | `A \| B` pattern |
| `pat.paren` | parenthesized pattern |
| `pat.path` | path pattern (unit variant or const) |
| `pat.range` | range pattern |
| `pat.reference` | `&pat` pattern |
| `pat.rest` | `..` pattern |
| `pat.slice` | slice pattern |
| `pat.struct` | struct pattern |
| `pat.tuple` | tuple pattern |
| `pat.tuple_struct` | tuple-struct pattern |
| `pat.type` | type ascription in `let` or a closure parameter |
| `pat.wild` | `_` pattern |
| `type.array` | array type |
| `type.bool` | `bool` type |
| `type.fn_ptr` | fn pointer type |
| `type.group` | invisible group type |
| `type.impl_trait.arg` | `impl Trait` outside a return type |
| `type.impl_trait.ret` | `impl Trait` in a return type |
| `type.infer` | `_` type |
| `type.never` | `!` type |
| `type.paren` | parenthesized type |
| `type.path` | path type other than `bool` |
| `type.path.qualified` | qualified path type `<T as Trait>::Assoc` |
| `type.ptr` | raw pointer type |
| `type.reference` | `&T` |
| `type.reference.mut` | `&mut T` |
| `type.slice` | slice type |
| `type.dyn` | trait object |
| `type.tuple` | tuple type |
| `type.unit` | `()` type |
| `type.lifetime` | lifetime in a type or bound |
| `type.lifetime.static_str` | the `'static` of `&'static str` |
| `generics.type_param` | generic type parameter |
| `generics.lifetime` | lifetime parameter |
| `generics.const_param` | const generic parameter |
| `generics.where` | where clause |
| `generics.hrtb` | `for<'a>` bound |
| `generics.assoc_binding` | associated item binding `Trait<Item = T>` |
| `generics.const_arg` | const generic argument |
| `attr.doc` | doc comment or `#[doc]` |
| `attr.derive` | derive of names listed in `derives` |
| `attr.cfg_test` | `#[cfg(test)]` or `#[test]` |
| `macro.allowed` | macro listed in `macros` (or `test_macros` in tests) |
| `unknown.item` | item kind the classifier does not know |
| `unknown.impl_item` | impl item kind the classifier does not know |
| `unknown.trait_item` | trait item kind the classifier does not know |
| `unknown.foreign_item` | extern block item kind the classifier does not know |
| `unknown.expr` | expression kind the classifier does not know |
| `unknown.pat` | pattern kind the classifier does not know |
| `unknown.type` | type kind the classifier does not know |
| `unknown.bound` | bound kind the classifier does not know |
| `unknown.generic_arg` | generic argument kind the classifier does not know |
| `unknown.attr` | attribute the classifier cannot read |
| `attr.derive.<Name>` | derive of a name not in `derives` |
| `attr.<name>` | attribute not in the subset |
| `macro.<path>` | macro not listed in `macros` |
| `type.interior_mut.<Name>` | std cell, lock or atomic type |
| `expr.interior_mut.<Name>` | std cell, lock or atomic type in an expression path |
