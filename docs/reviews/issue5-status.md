# Conductor milestone 2 friction after subset v14

This review continues [issue #5](https://github.com/JimLundin/subset/issues/5)
against subset `64352fdd5935de3954ee06b4598789cb89f67a03` (configuration
version 14) and Conductor `79568ce8a55b266fe1cf96b17dd5b2bc2dfa0c8c`.
The original issue combines requests from several rounds; their original
friction logs were not all retained. A missing original example is not
evidence that a new rule is needed.

## Work covered by this review

| Reported friction | Disposition |
|---|---|
| Repeated local constructors and path-error mappings | Proposal 0046 already permits confined immutable local closures. Use them for repetition within a function; shared shapes across functions remain free functions. |
| Sharing three approval button bodies | A shared free function is already permitted. Sharing a captured, escaping local closure is a separate policy question. |
| Literal arrays as method receivers | Proposals 0047 and 0050 already permit these. Remove wrappers that only convert an array before an existing receiver call. |
| Duplicate live event arms | Proposal 0048 permits `kind @ (A { .. } \| B { .. })`. Preserve event ordering and sequence behavior when combining the arms. |
| Constant numeric bounds | Proposal 0049 permits range patterns. Keep the same inclusive and exclusive bounds and the explicit fallback. |
| Copy values behind references | Proposal 0053 permits shared-reference patterns. A pattern can extract the value where a general dereference expression remains outside the subset. |
| Wire boolean patterns, Serde defaults/fallbacks, embedded text, typed causes | Already addressed by proposals 0054–0057. These scoped permissions do not admit domain boolean flags or arbitrary Serde syntax. |
| `#[cfg(test)]` re-export | Already allowed as `attr.cfg_test`; it is not subject to the platform counterpart rule. |
| Platform counterparts in unrelated scopes | Checker correctness work: an unrelated function or method must not satisfy another item's required counterpart. Existing predicates and module exceptions remain in force. |
| Synchronous channel draining through an async stream | Accepted and implemented by proposal 0058 at version 15: one immediate, nonblocking `std::iter::from_fn(...).find_map(...)` shape. |
| Path-error mappings shared across functions | Proposal 0059 would permit private, immediately consumed `FnOnce` factories with existing immutable path arguments. Nine sites remain after 0046. The owner chose to keep it proposed for now. |
| Returned closures, blocking callbacks, actor initialization and platform predicates | Evaluate separately with the full behavior and all affected sites. A simpler spelling alone does not establish ownership, cancellation, ordering, or portability equivalence. |
| Domain booleans, arbitrary mutation, indexing and early `return` | Existing named-state, forward-flow and error-as-value rules still apply. The report does not authorize these constructs. |

## Evidence and decisions

The companion reports cover [callbacks and actors](issue5-callbacks-actors.md),
[comparisons and references](issue5-comparisons-references.md),
[platform attributes](issue5-platform-attributes.md), and
[returned error helpers](issue5-error-factory.md). They distinguish supported
rewrites, checker defects, proposed extensions, and cases without enough
evidence. The code changes receive independent review and the repositories'
normal checks before delivery.

Reported savings compare the stated candidate rewrites at the pinned
baseline. They are not whole-repository savings for the final consumer
change, which also adds regression coverage.

## Checker correction

Complementary platform attributes must belong to the same logical item.
The checker now distinguishes modules, ordinary blocks, impl owners, and
block-local type declarations. It recognizes equivalent ordinary import
paths, grouped module imports, raw identifiers, and consistently renamed
generic parameters, including reordered blanket binders. This corrects
enforcement of accepted proposal 0016 without adding predicates or syntax.
It was delivered at version 14 before the separate 0058 extension.

The regression fixture contains 58 accepted platform attributes and 28
rejected attributes without valid counterparts. The 11 integration groups
include all 95 CLI case directories and native compilation of the actual
positive owner examples. Rust name resolution beyond the recorded ordinary
imports and visible declarations remains outside this syntactic checker;
the [platform report](issue5-platform-attributes.md) states those limits.
The [independent review](issue5-checker-review.md) and
[integrated verification](issue5-checker-verification.md) cover the
delivered correction and the test scratch-directory race found during
parallel validation.

## Consumer delivery and validation

The [reviewed Conductor patch](issue5-consumer/conductor.diff) applies the
already accepted v14 forms across 15 Rust files. The
[independent review](issue5-consumer-review.md) found no required code
fixes. The [verification record](issue5-consumer/verification.md) records
passing formatting, native and wasm clippy, 96 non-endpoint tests, workspace
doc tests, and the pinned subset check on the final patch.

The initial complete Conductor gate did **not pass**: daemon endpoint tests
stalled in WSL kernel network enumeration before fake CLI startup.
Unrelated processes exhibited the same wait. The owner authorized an
Ubuntu WSL restart on 2026-11-13; the fresh kernel and disappearance of the
blocked processes were verified. The initial v14 patch and evidence remain
retained here as the reviewed baseline.

The combined 16-file Rust patch has passed independent review and all 31
prescribed live checks (16 Claude and 15 Codex). These live checks used the
user's normal login environment; initial invocations without that
environment lacked the required provider configuration. Raw recordings
remain local.

While the version-15 checker was being reviewed, separate work integrated
the consumer changes with newer feature work in Conductor
`a9e607dd3c93b9e7e0da5bf8116641e777b01aae` and removed the original temporary
worktree. That commit has version-15 configuration but retains the old
checker pin. The newly built version-15 checker passes on that integrated
main.

Conductor commit `6b462067af97769ad9c3295be015abb979df35f5` now pins published
subset `b02997b97b1db70bdc12ad903ca433ca5dc094cc`. This final update changes
only `tools/subset` and roadmap text; integrated Rust and configuration
remain unchanged. The [pin review](issue5-consumer-v15-review.md) and
[delivery record](issue5-consumer/v15-verification.md) record the result.

The owner explicitly declined the full suite for this pin-only update.
The running gate was stopped and no further tests or builds were run.
Delivery uses the completed pin review, revision verification, and check
of the integrated source with the version-15 checker. The full pinned
wrapper gate did not finish. The earlier Rust review and 31 live checks
remain evidence for the original reviewed patch.

## Extension decisions

Both proposals have independent reviews. On 2026-11-13 the owner accepted
0058 and chose to keep 0059 proposed for now.

| Proposal | Evidence | Decision |
|---|---|---|
| [0058: synchronous receiver iterator](../proposals/0058-synchronous-receiver-iterator.md) | Affected score 131 to 100.5 (23.3% lower), 26 tokens saved; both versions pass agents compilation, three channel tests and two startup-refusal replay tests. Existing tolerated closure tier; library boundary pricing applies. [Review](issue5-proposal-review.md). | Accepted and implemented at version 15. |
| [0059: path-error factory](../proposals/0059-path-error-factory.md) | Nine remaining friction sites; affected score 1122 to 1095.5 (2.36% lower), 35 tokens saved. Focused clippy, six behavior tests, negative ownership checks and actual agents compilation pass. Tolerated tier; no boundary pricing for the factory definition. [Review](issue5-error-factory-review.md). | Keep proposed for now; no implementation. |

Issue #5 remains open while its remaining rule proposals or evidence gaps
are unresolved. No candidate becomes accepted merely because its report or
prototype is committed.
