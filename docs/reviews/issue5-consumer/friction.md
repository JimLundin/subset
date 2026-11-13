# Remaining friction after the subset v14 simplifications

These sites remain in the published Conductor code. The rewrites in this
round use already accepted constructs; they introduce no new syntax or
startup, cancellation, ordering, or provider protocol behavior.

## Actor-loop setup

- `crates/daemon/src/session.rs:231` (`Session::spawn`) and `:432`
  (`SessionActor::run`): initialization would read directly before the run
  loop. The current rule requires the loop to be the whole function body.
  The code instead enqueues `SessionMsg::Start` with a discarded reply
  channel during spawning, then processes it in the actor's FIFO. This
  round preserves that startup path.
- `crates/agents/src/install.rs:297` (`drain`): wanted one local
  `let write_error = |error| io(to, error);` before the loop, passed to both
  `map_err` calls. The whole-body-loop rule prevents this setup, so the two
  inline closures remain. Putting setup inside the loop would repeat it
  on each chunk.

## Synchronous predicate iteration

- `crates/agents/src/claude/session.rs:468` (`refusal`): wanted synchronous
  `std::iter::from_fn` over `try_recv`. It is not an accepted closure-taking
  call. The code instead uses `Box::pin(stream::unfold(...))` and awaits
  its first matching event. Any new form needs its own accepted proposal.

## Blocking-library calls

- `crates/agents/src/install.rs:190` (`hold`) and `:201` (`attempt`): wanted
  a confined `spawn_blocking` closure around the advisory file lock. This
  call is not accepted. The code instead polls `try_lock` in its loop
  and sleeps between attempts. Cancellation and lock ownership must be
  considered in a separate proposal before changing this behavior.

## Escaping closure abstraction

- `crates/agents/src/install.rs:226` (`remove`), `:235` (`unpack`), `:278`
  (`fetch`), `:297` (`drain`), and `:314` (`sha256`): a function returning
  `io_error(path)` could replace the repeated path-to-error adapters across
  functions. It would require an escaping closure, an `impl Trait` return,
  and a captured reference lifetime, none accepted for this purpose. The
  shared `io(path, error)` free function remains; confined local closures
  now share the repeated same-block adapters in `unpack` and `sha256`.

No concrete site requiring multiple mutable domain loop-state bindings was
found at the published revision. That item from issue #5 needs fresh code
evidence before a proposal.
