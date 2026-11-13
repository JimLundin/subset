# Independent review of accepted 0058: approved

Approved immutable candidate: `a555f8f5468daef2142a1c2737e2c5c1acad5e73`.

Reviewed the correction delta from `031a0c64294947537d8981b7da378f1e0bde4efa`, together with the completed full implementation review versus base `ed855575da227bf66f270b5759c3beb7bbd4af8b`. Assessment used immutable git show/diff, actual source/fixtures, accepted proposal and active policy, without implementer private reasoning. All previously batched required findings are resolved. No new required correctness or regression issue was identified in the scope/import changes.

## Required-fix resolutions confirmed

Receiver origins now live in lexical Cx binding maps. Parameters and locals update at their actual binding points, and blocks/arms/closures restore their enclosing maps. This prevents an unrelated inner or later own binding from changing an external parameter's classification. Shadowing in closures, match arms, if-let bodies and loops follows the appropriate scope.

Direct self and Self use the actual impl owner. Supported tuple parameters retain each declared binding's type, including suffix positions after rest. Raw names are canonicalized consistently. Ordinary aliases/references preserve an already known origin, and typed local closures classify their declared parameters. Positive/negative fixtures cover own self/Self, raw names, tuple parameters, aliases, own shadows, external shadows and out-of-scope/later bindings.

Constructor identity now resolves through the pre-gathered scoped declaration/import metadata, including grouped-self imports and raw identifiers. Calls through aliases are recognized for rejection while admission still requires the direct std constructor path. Named callbacks, constructor values and escaping/lazy generators remain outside. Grouped/raw alias and function-value fixtures cover the earlier escape examples.

The special rules apply to the actual library constructor. Ordinary calls, imports and function values of a local same-named function retain the existing core rules; its unsupported inline callback still fails generic closure checking. An explicit ::std root bypasses local std shadows and receives the existing library boundary mark. The shadow fixture now includes positive local-core operations and the absolute external drain, plus negative local/raw-shadow callbacks. Its construct score explicitly verifies the closure boundary.

## Accepted scope and pricing

The exact inline zero-argument synchronous receiver.try_recv().ok() pull remains immediately consumed by inline eager find_map. Parentheses and move remain accepted. Named callbacks on recognized constructors, blocking pulls, extra arguments/generics, malformed bodies, alternate terminals, intervening adapters and retained/returned/passed generators remain rejected. The general closure callee list is not broadened, preserving 0046 confinement and the existing async stream forms.

Version 15 and accepted 0058 are reflected consistently in the policy/docs. 0059 remains proposed with no checker or allowlist admission. The coordinator's known final proposal-index/consumer publication update is outside this source review and was not treated as a blocker.

The existing CallArg library mark and tolerated expr.closure tier correctly price the two pull delimiter tokens at boundary 1.0. The retained full-function regression continues to assert before 131.0/143/18 and after 100.5/117/13. No scoring implementation or weight change is needed. Static inspection found no new subset/self-hosting construct breach.

## Validation limits

Immutable git diff --check passed against the specified implementation base. The implementer reports fmt, clippy all targets with warnings denied, all 12 integration groups and self-check passing, including the final borrowed-receiver fixture adjustment. This reviewer did not run duplicate builds/tests. Approval applies to the exact candidate hash above; the coordinator owns the final integrated and publication gates.

Type origin of unresolved bindings/fields, external method semantics, macro expansion and compiler type resolution remain the documented review/compiler limits. They were not treated as additional admission or new requirements.
