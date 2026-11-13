# Verification snapshot before WSL recovery

Source diff is stable and independently reviewed. Linux git status contains
exactly the 15 intended Rust source files;
`CLAUDE.md` is unchanged. `cargo fmt` and `git diff --check` passed. The first
early subset v14 check passed before the final literal-array wrapper edits.

The full `tools/check.sh` ran with the required PATH and this checkout's
own target directory. Native and wasm clippy passed. Workspace unit/replay
tests progressed to the daemon e2e suite, where all eight initial test
threads stopped before producing a test result. The full log is preserved
at `.audit/check-initial.log`; the stopped gate's original `check.log` was
removed after archiving it.

Observed process tree:

- 1991424: owning `bash -lc` gate command.
- 1991433: `bash tools/check.sh`.
- 2010277: `cargo test --workspace`.
- 2015871: this checkout's daemon e2e binary.
- 2015872 through 2015879: the eight test threads, all uninterruptible
  `D` state with wait channel `rtnl_dumpit`.
- The e2e main thread waits in `futex_do_wait`. No fake CLI children started.

At inspection WSL had about 6.7 GB available memory. Independent existing
processes `xdg-desktop-portal` (95580, alive for about 1 day 18 hours) and
`python3` (1998633) were also in `D/rtnl_dumpit`. The published 79568ce
endpoint setup and all eight stalled e2e test bodies are unchanged by this
diff, establishing a shared kernel/network-interface enumeration blocker.

The coordinator authorized termination of only this gate. Every PID's
command and exact owning-worktree cwd was checked before SIGTERM. The gate
and cargo parents exited, releasing the Cargo lock; the e2e leader became a
zombie. No unrelated process was signaled.

Independent non-endpoint verification then completed with
`CARGO_BUILD_JOBS=2` and `RUST_TEST_THREADS=2`:

- `cargo test -p proto`: 11 tests passed, including frame-limit boundaries.
  See `.audit/check-proto.log`.
- `cargo test --workspace --doc`: passed. See `.audit/check-doc.log`.
- Pinned `tools/subset check`: passed on the final source diff. The checker
  was installed through the pinned wrapper into this checkout's own target.
  See `.audit/check-subset.log` (silent success), and the zero exit status of
  `.audit/independent-checks.sh`.
- Earlier native/wasm clippy and 85 unit/replay tests passed on the final
  source, giving 96 passed non-endpoint tests in total.

The full gate remains blocked solely at the daemon e2e suite; it has not
passed. No source edits, commits, new endpoint tests, or duplicate gates were
made while WSL recovery permission is pending. After recovery, rerun the
full gate on the final diff with both limits above and pinned `tools/subset`.
