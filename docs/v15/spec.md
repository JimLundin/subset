# The subset, version 15 (target)

This is the language the owner ruled on 2026-11-13, recorded in
[issue #24](https://github.com/JimLundin/subset/issues/24). It replaces
`docs/subset.md` when the work in [`plan.md`](plan.md) lands. Until then the
checker enforces version 14 and this page is the target.

Anything not listed here is out. Where this page is silent on a construct
that version 14 ruled on, version 14's status holds; the notable cases are
in [Carried over](#carried-over).

## Principles

1. **The reader comes first.** A function is understood from its own text
   by someone who didn't write it. Readability beats brevity when they
   disagree.
2. **It fits in your head.** The definition is one page, and each construct
   has a one-sentence rule.
3. **One form per job, where possible.** Where two forms overlap, the score
   says which is preferred in which circumstance. That is all the score is
   for.
4. **Nothing is decoded.** Meaning lives in names and types, not in flags,
   sentinels or positions.
5. **A name means one thing.** Data is immutable, and a name is never
   shadowed, reused or overwritten.
6. **One purpose per item.** A function or type does one thing that a short
   name can state.
7. **Rules are mechanical.** What the checker can't enforce from syntax is
   guidance, not subset.

The subset is about readability. Freedom from panics and the architecture of
one project are not its goals: arithmetic is in the core, and nothing in the
core names a library.

## The language by job

### Define data

| Sub-job | Form |
|---|---|
| A thing with named parts | `struct` with named fields |
| A value with an identity or unit | newtype, `struct HostSeq(u64)` |
| One of several cases | `enum`, each variant carrying its own data |
| The same shape over different types | unbounded type parameter, `Step<Id>` |
| Several values of one type, fixed count | array |

- No tuple of ours: never a field, parameter or return type, never built as
  a value. See [Choose](#choose) and [Repeat](#repeat) for the two places a
  tuple pattern appears.
- `.0` only inside a newtype's own methods.
- `bool` is never a field or a parameter.
- No trait definitions and no type aliases.

### Name things

| Sub-job | Form |
|---|---|
| Name a value | `let name = value;`, immutable |
| Take a value through steps | a chain, or a new name per step |
| Name a fixed value | `const NAME` |
| Throw a value away | `let _ = value;` |

- **No shadowing.** A name is not rebound while an earlier binding of it is
  in scope: not by a `let`, a parameter, a closure parameter or a pattern.
  Sibling scopes may reuse a name, so two match arms may both bind `id`.
- A local is never assigned to.

### Compute

Calls, method calls, field access, `&`, and the arithmetic and comparison
operators (`+ - * / %`, `== != < <= > >=`, `&& || !`).

- A boolean expression stands only as the condition of an `if`, a match
  guard, the body of a predicate closure, or the body of a predicate
  function. It is not bound to a name.
- A container is never indexed. It is iterated or matched.

### Choose

| Sub-job | Form |
|---|---|
| Pick among two or more cases | `match`, every case of our own enums named |
| Pick on several values at once | `match (state, event) { (State::Idle, Event::Start) => .., .. }` |
| Act on one case, otherwise nothing | `if let` with no `else` |
| Pick between two values on a condition | `if cond { a } else { b }` |
| Do something only on a condition | `if cond { effect }` |
| Use a plain default when absent | `unwrap_or(value)` |

- An `if` that produces a value has both branches. There is no `else if`:
  three or more outcomes are a `match`.
- A `match` over an enum of ours has no catch-all `_` arm.
- A guard stays where an arm's pattern and a condition combine. A `match`
  that exists only to test a comparison is an `if`.
- A predicate function returns `bool`, has a name that asks the question
  (`is_empty`), and has a comparison, a `matches!` or a predicate call as
  its body.
- No `true` or `false`, with one exception under
  [Library boundary](#library-boundary).
- Not forms: `map_or`, `map_or_else`, `unwrap_or_else`, `is_some_and` and
  its siblings, `.then(..)`, `.then_some(..)`, `Some(x).filter(..)`, and
  `Option<()>` as a type of ours. Each is a `match` or an `if`.

### Repeat

| Sub-job | Form |
|---|---|
| Transform, select or collect items | adapter chain (`map`, `filter`, `collect`) |
| Reduce items to one value | named adapter (`sum`, `count`, `any`, `max`, `find`), else `fold` |
| Run an effect per item | `for item in list { effect(item); }` |
| Count through numbers | range, `0..n` |
| Wait for events until done | `loop`, as the body of an actor's `run` |

- A closure in a chain is one expression. Anything bigger is a named
  function.
- A pair a library yields (`enumerate`, `zip`, map entries) is taken apart
  at once into names: `|(index, item)|`.
- An element is named by iteration or by a slice pattern,
  `[head, rest @ ..]`.
- Not forms: `while`, `for_each`, `continue`, `break` outside the actor
  loop.

### Fail

| Sub-job | Form |
|---|---|
| Something can fail or be absent | return `Result` or `Option` |
| Define what can go wrong | an `enum` of errors, one message per variant |
| Pass a failure to the caller | `?` |
| Turn absence into an error | `ok_or(..)` |
| Turn one error into another | `map_err(..)` at the call |
| Assert that something cannot fail | `expect("why")` |
| Ignore a failure on purpose | `let _ = ..;` |

- No `unwrap` outside test code.
- No `#[from]`: a conversion is written where it happens.

### Change state

| Sub-job | Form |
|---|---|
| State that changes over time | a struct with `&mut self` methods |
| Set a field to a new value | `self.field = value;` |
| Change a field in terms of itself | `self.count += 1;` |
| Lend a value to a library that demands `&mut` | `let mut name`, never reassigned |

- Assignment is only to `self.field`, in a `&mut self` method.
- A type with `&mut self` methods does not derive `Clone`. That is the line
  between state and data.
- `self.count = self.count + 1` is written `self.count += 1`.
- No interior mutability.

### Wait

| Sub-job | Form |
|---|---|
| Do I/O without blocking | `async fn` and `.await` |
| Run something alongside | an `async` block passed to `spawn` |
| Wait for whichever event comes first | `select!` inside the actor `loop` |

### Library boundary

A shape a library demands is allowed at the point where it is handed to
that library, and nowhere else.

- The project's `subset.toml` lists the derives, attributes, macros, library
  traits and closure-taking calls it uses, and the paths each is allowed
  under. The checker has no library's name compiled in.
- A boolean literal appears only as the value of a named constant,
  `const CLOSED: bool = false;`, used as `watch::channel(CLOSED)`.
- A flag read from a wire type goes straight into an `if`.

## Shape limits

| Limit | Value |
|---|---|
| Nesting depth of blocks inside a function | 3 |
| Lines in a function | 40, unless it is a dispatcher |
| Parameters besides `self` | 4 |
| Words in a name | 3, with no `and` or `or`; test names exempt |

A dispatcher is a function whose body is a single `match` or `select!` with
arms of one expression or call each.

## The score

The score ranks forms that do the same job. Forms with different jobs carry
no weight against each other. The overlaps it decides:

| Overlap | Preferred |
|---|---|
| a named reducer or `fold` | the named reducer |
| a slice pattern or `first()` / `last()` | the slice pattern |
| a guard or a choice nested in the arm | the guard when the condition selects the arm |
| a primitive or a newtype in a field or signature of ours | the newtype |

`unwrap_or(value)` against a two-arm `match` is a rule, not a weight: the
argument of `unwrap_or` is a literal, a constant or a path, and anything
computed is a `match`.

## Carried over

Version 14's status holds for these, which the rulings did not touch:
references and slices, `&'static str` in scanned tables, local closures,
struct update, `cfg` pairs, visibility (`pub`, `pub(crate)`), the pattern
forms (`@`, ranges, slice and reference patterns), the actor `loop` rules,
`drop` and discard rules, consuming readers, unit `()` at a boundary, and
what stays out (`as` casts, `return`, `let .. else`, `unsafe`).

## What left, from version 14

- The ban on arithmetic operators and their trait names (0038), and ranges
  built by name.
- The ownership analysis for `&mut self` (0008), the `endpoints` list (0035)
  and the `Visit` exception (0045): one rule replaces them.
- The body rules of a `for` (0041): with no mutable locals a `for` cannot
  build a local value.
- `#[from]` (0011), tuples as short-lived pairs (0001), comparison-only
  guards (0040) and the boolean-expression confinement (0028) in their old
  forms.
- Every library name in the checker's source.
