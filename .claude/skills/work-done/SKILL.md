---
name: work-done
description: Derive Decision Records and Design Document updates from a finished Work Log, its plan files, and the uncommitted changes, then retire the working records. Use when a unit of work is implemented and ready to be written up.
argument-hint: "work log filename — or leave empty to use the most recent"
disable-model-invocation: true
allowed-tools: Bash(date *) Bash(git branch *) Bash(git status *) Bash(git diff *) Bash(ls *) Bash(grep *) Read Write Edit
---

# Close out a unit of work

## Project state

- Today: !`date +%Y-%m-%d`
- Branch: !`git branch --show-current 2>/dev/null || echo "(not a git repository)"`
- Work Logs: !`ls -1 docs/work/ 2>/dev/null || echo "(none)"`
- Plans: !`ls -1 docs/plans/ 2>/dev/null || echo "(none)"`
- Design Documents: !`ls -1 docs/design/ 2>/dev/null || echo "(none)"`
- Decision Records: !`ls -1 docs/decisions/ 2>/dev/null || echo "(none)"`
- Uncommitted changes: !`git status --short 2>/dev/null || echo "(none)"`
- Diff summary: !`git diff HEAD --stat 2>/dev/null || echo "(none)"`

## Target

$ARGUMENTS

## Sources, and what each one is good for

Identify the Work Log named above, or the most recent one if nothing was named,
and the plan files in `docs/plans/` belonging to the same work. Read them, then
read the uncommitted changes with `git diff HEAD` — read it by path rather than
all at once if the summary above shows it is large.

Each source answers a different question, and confusing them produces bad
documents:

The **Work Log** holds what was asked and how the understanding of it evolved.
The Request section is the only record of the original wording.

The **plans** hold what was intended at the moment they were written. They are
aspirational. Never copy plan text into a Design Document as though it describes
reality.

The **diff** is the only source that says what actually exists. When a Design
Document and the code disagree, the code wins and the document is wrong.

## Find the decisions

Start from the divergences. Where the code differs from what the plan said, or
from what the Work Log's earlier entries assumed, something was decided during
implementation — and decisions made mid-flight are exactly the ones nobody
remembers to write down. List every such divergence before evaluating any of
them.

Then apply the bar. A Decision Record is warranted when the decision has durable
consequences, when meaningful alternatives existed, when a real trade-off was
accepted, or when reversing it would be costly.

Write one also for knowledge that has no home in a Design Document but should
outlive this work: an approach that was tried and abandoned, a constraint that is
not visible from reading the code, a reason something is *not* done the obvious
way. Design Documents describe what the system is; they cannot hold what it is
not, and this working record is about to be deleted.

Routine implementation choices do not warrant a record. Do not pad the set.

**Present the candidate list and wait for confirmation before writing anything.**
For each candidate give one line: the decision, and which test above it meets.

## Write the Decision Records

Number sequentially from the highest existing record. Never renumber; other
documents cite these by identifier.

```markdown
# DR-NNNN: <decision, stated as a sentence>

Status: accepted
Date: YYYY-MM-DD

## Context
The situation and the forces that made a decision necessary.

## Decision
What was decided.

## Alternatives
What else was considered, and why it was not chosen.

## Consequences
What this makes easy, what it makes hard, what reversing it would cost.
```

Write the reasoning as it stood when the decision was made, not as it looks with
hindsight. If this work reverses an existing decision, write the new record and
ask before touching the old one — the only permitted edit to an existing record
is its Status line, changed to `superseded by DR-NNNN`.

## Update the Design Documents

Rewrite the affected documents so that they read as a description of the present.
Take the facts from the diff, not from the plan.

A Design Document is not a changelog. Remove any wording that only makes sense to
someone who knows the previous state — no "now uses", no "changed from", no
"previously". A reader arriving fresh must not be able to tell that anything was
ever different.

Where the current shape exists for a non-obvious reason, state the constraint
plainly and cite the Decision Record. Do not retell the story in the Design
Document; that is what the citation is for.

Updating these documents overwrites existing content, and an overwrite can
quietly erase intent nobody remembers to restore. **Show the proposed changes and
get confirmation before applying them.**

If the work added a new area, create a new document and add it to
`docs/design/index.md`.

## Verify the retirement checklist

Check each item against the files. Do not assume.

- Does every behavioural change in the diff appear in a Design Document?
- Is every confirmed decision recorded, with its number?
- Would anything worth keeping vanish if the Work Log and plans were deleted
  right now? Re-read the Work Log's Progress section with this question in mind.
- Does any durable document reference the Work Log or a plan file? Grep
  `docs/design/` and `docs/decisions/` for their filenames. Any hit must be
  rewritten to stand on its own.

Report the result item by item. If something fails, fix it and check again.

## Retire the working records

Only once every item passes, and only with explicit confirmation: mark the Work
Log `Status: complete`, then delete it and its plan files. They remain in version
control, so this removes them from the working tree rather than destroying them.

If the user prefers to keep them until the branch merges, that is fine — say so
and stop. The checklist passing is what matters; the deletion is bookkeeping.
