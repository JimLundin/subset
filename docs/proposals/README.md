# Subset proposals

One file per proposal, numbered in order: `NNNN-<construct-id>.md`. The template is in `.claude/skills/pair/SKILL.md`.

A case filed from another project, as a friction report or a candidate,
follows [Presenting a case](../presenting-a-case.md).

## Open requests

- **Type parameters on internal step types** (issue #1, item 3, low
  priority). Keeping provider ids typed between the daemon's mapping and
  its session actor would need a type parameter on `Step`; 0013 allows
  type parameters only on codec fns, so the ids became text newtypes.
  Needs measurements from the daemon before it can be proposed.
