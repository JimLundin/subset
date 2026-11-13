# Integrated version-15 verification

The coordinator validated subset source commit
`08992fbc9870a33d5777c4623ac4f2fb07553e99`, combining the independently
reviewed cfg correction, test scratch isolation, and accepted 0058.
Validation used `CARGO_BUILD_JOBS=2` and `RUST_TEST_THREADS=2`.

All subset gates passed:

- `cargo fmt --all --check`
- `cargo test --offline`: all 12 integration groups, including 98 CLI case
  directories and the retained Conductor measurement regression.
- `cargo clippy --offline --all-targets -- -D warnings`
- `target/debug/subset check` against this repository.

The measurement regression checks complete before and after functions:
score 131.0 to 100.5, tokens 143 to 117, and measured code lines 18 to 13.
The actual library constructor retains boundary pricing. The scoped
receiver-origin, alias, raw-name, and local/absolute callee cases are
covered by the CLI fixtures and independent implementation review.

The scripted final consumer check could not enter the original temporary
worktree because another process had already integrated and removed it.
The coordinator inspected the new clean Conductor main at
`a9e607dd3c93b9e7e0da5bf8116641e777b01aae`, confirmed the integrated 0058
rewrite and version-15 configuration, and ran the newly built checker
against that checkout instead. It exited 0. No old consumer patch was
reapplied and no integrated feature work was replaced.

The consumer's published pin and full gate are recorded separately after
the version-15 checker revision is available. Proposal 0059 remains
proposed at the owner's request.
