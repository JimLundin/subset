# Integrated cfg checker verification

The coordinator validated integrated commit
`ed855575da227bf66f270b5759c3beb7bbd4af8b` after the independent checker
review and the separate review of the test scratch-directory correction.
Ubuntu WSL had been restarted with the owner's permission; these checks
used `CARGO_BUILD_JOBS=2` and `RUST_TEST_THREADS=2`.

All commands exited successfully:

- `cargo fmt --all --check`
- `cargo test --offline`: all 11 integration groups passed, including all
  95 CLI case directories and native compilation of the cfg owner examples.
- `cargo clippy --offline --all-targets -- -D warnings`
- `target/debug/subset check` against this repository.
- The same built checker against the reviewed Conductor v14 patch in
  `/home/jim/projects/apps/conductor-issue5-v14`, before adding accepted 0058.

The earlier integrated test run exposed an existing race: the CLI case
runner and the boundary-data compiler test both used the same temporary
directory within one test process. The case runner could remove
`pins.toml` while the compiler read it. Commit `78f88080` prefixes CLI case
scratch names with `case-`, separating them from compiler-test scratch
directories. The final parallel run above includes that correction.

This verification covers the cfg correction at configuration version 14.
The owner subsequently accepted proposal 0058 for implementation; its
version-15 checker and consumer rewrite require their own verification.
Proposal 0059 remains proposed at the owner's request.
