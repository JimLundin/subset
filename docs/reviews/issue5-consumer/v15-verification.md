# Version-15 consumer delivery

Published Conductor commit:
`6b462067af97769ad9c3295be015abb979df35f5`.
Pinned subset revision:
`b02997b97b1db70bdc12ad903ca433ca5dc094cc` (version 15).

The pin update changes only `tools/subset` and `docs/roadmap.md` relative
to integrated Conductor main `a9e607dd3c93b9e7e0da5bf8116641e777b01aae`.
Rust source and version-15 configuration remain unchanged. The
[independent review](../issue5-consumer-v15-review.md) approved the exact
two-file diff, verified the published revision and policy, and confirmed
that the accepted 0058 drain remains present while 0059 remains proposed.
The coordinator's already-built version-15 checker accepted integrated
main before the pin update.

## Owner-directed validation scope

The owner explicitly instructed that this pin-only update must not run
the entire test suite. That direction supersedes the repository's blanket
gate rule. The running gate was stopped, and no further tests or builds
were run. Delivery used the completed focused evidence above.

The existing log shows that formatting, native clippy, the three wasm
clippy stages, 181 workspace tests and doc tests had already completed
before the stop. The final pinned-checker installation was compiling when
the coordinator stopped the gate. The complete `tools/check.sh` therefore
did **not** finish; this record does not claim otherwise.

The consumer agent verified the stopped process tree belonged to the
temporary pin checkout before sending SIGTERM. Its log, process record,
pending diff, and stage summary are retained locally under Conductor's
`.audit/issue5-v15-pin-6b46206`. The temporary checkout and branch were
removed after publication, and primary `main` was fast-forwarded.

## Earlier Rust and live-check evidence

The original combined cleanup/0058 Rust patch had independent review and
passed all 31 prescribed live checks: 16 Claude and 15 Codex. These checks
used the user's normal login environment after initial invocations lacked
provider configuration. Raw logs and recordings remain local in
Conductor's `.audit/issue5-original`.

Separate work subsequently integrated that patch with newer features in
`a9e607d`. The original review and live results apply to their recorded
patch; no integrated live rerun is claimed for this pin-only delivery.
