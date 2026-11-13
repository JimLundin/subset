# Subset proposals

One file per proposal, numbered in order: `NNNN-<construct-id>.md`. The template is in `.claude/skills/pair/SKILL.md`.

A case filed from another project, as a friction report or a candidate,
follows [Presenting a case](../presenting-a-case.md).

## Owner-directed additions

- **Patterns, immutable data, and strong domain models**
  ([issue #6](https://github.com/JimLundin/subset/issues/6)). The owner's
  broader direction, with concrete examples: range/reference patterns,
  immutable fixed-size domain values, borrowed slice returns, generic
  internal models and boundary data. Versions 12/13 implement receivers,
  grouping, `@`, slice patterns and ranges (0047–0049). Version 14 completes
  the approved remainder: [domain arrays](0050-immutable-domain-arrays.md),
  [shared slice outputs](0051-shared-slice-outputs.md),
  [generic models](0052-generic-domain-models.md),
  [reference/rest patterns](0053-reference-and-rest-patterns.md),
  [wire boolean patterns](0054-wire-boolean-patterns.md),
  [Serde defaults/fallbacks](0055-serde-default-and-other.md),
  [embedded strings](0056-embedded-strings.md), and
  [typed error sources](0057-typed-error-sources.md).

## Open requests
- **Remaining Conductor milestone 2 suggestions**
  ([issue #5](https://github.com/JimLundin/subset/issues/5)). Record the
  owner's approval to consider the reported improvements: additional
  closure-taking calls and callback sharing, comparisons and guards,
  dereferencing, actor setup and platform attributes. Local closures
  (0046), array receivers (0047), grouped `@` alternatives (0048), ranges
  (0049), embedded strings (0056), typed causes (0057) and scoped boundary
  Serde/boolean syntax (0054/0055) now have rules. General dereference
  expressions, more callback APIs, runtime comparisons and actor/platform
  changes still need their own rules and evidence. The reported examples
  do not all pass merely because these narrower forms are implemented.
