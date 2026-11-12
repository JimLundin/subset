# Presenting a case for changing the subset

How to file friction or a candidate construct from a project that uses the
subset, so that a reviewer here can evaluate it from the issue alone.

## Before you file

Check that it hasn't been decided. Search `docs/proposals/` (rejected ones
too), `docs/bootstrap-review.md`, `docs/compromises.md` and "Not in the
core" in `docs/subset.md`. A decided question is reopened only with
evidence the decision didn't have, and the issue says which decision it
reopens and what is new.

## Two kinds of issue

**A friction report** says where the subset made code longer. It is cheap
and asks for nothing. **A candidate** asks for a change and carries the
measurements a proposal needs. File a report when you have sites but no
rewrite; several reports of one kind are what turn into a candidate.

## Every issue states

- The project and its commit, and the subset revision and `subset.toml`
  version it was checked with.
- One kind of friction per issue. Two constructs are two issues.

## A friction point

For each site:

- `path:line` at the stated commit.
- **Wrote:** the code as it is, verbatim, as a whole item (the fn, the
  arm with its `match`), not a fragment.
- **Wanted:** what the author would have written, as code.
- **Why not:** the rule that stopped it, by id or proposal number, or the
  line `subset check` printed.

Then, for the kind as a whole: how many sites the project has, with the
command that finds them (`grep -rn "then_some" crates`), so the count can
be checked.

## A candidate adds

1. **The rule**, as narrow as the sites need: where the construct is
   allowed, what the one form is, and which current form it replaces.
2. **Before and after on the same real code.** Apply the rewrite in a
   scratch copy. Say what was run on it and what passed (`cargo clippy
   --all-targets -- -D warnings`, the tests). Include the diff.
3. **`subset check` on the after**, verbatim. It shows which ids the rule
   has to admit.
4. **Measurements**, from `subset score --by file` and `subset measure`:
   - the affected code alone, before and after;
   - the whole files, before and after, so the reader sees how much of
     the file the change touches;
   - the after priced at the proposed tier. `subset score` prices a
     construct that isn't allowed yet as outside (5.0), so show the
     arithmetic that reprices it.
5. **Which test it meets**: a drop of at least 15% in the weighted score
   of the affected code, or at least three friction points of one kind.
6. **The seven qualities**, each kept or weakened with a one-line reason.
   Say so plainly when one is weakened. That is the owner's call to make,
   not a reason to leave the line out.
7. **The tool change** the rule needs in the checker.
8. **What was not measured**: other crates, sites where the rewrite makes
   the code longer, anything assumed.

A candidate that qualifies becomes a proposal in `docs/proposals/`, in the
template in `.claude/skills/pair/SKILL.md`. Its sections are the same
ones, so the issue's text moves over as it is.

## What makes a case hard to evaluate

- Invented code. Use the code that exists, at a commit.
- Only the best site. Show every site, including one that gets worse.
- An after that was never compiled.
- Numbers without the command that reprints them.
- A percentage of the helper alone, with the call sites left out.

Issue #3 (one `ask` helper per crate) is a candidate written this way.
