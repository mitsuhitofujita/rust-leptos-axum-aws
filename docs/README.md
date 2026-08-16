# Documentation Model

This project keeps two layers of documentation.

**Working records** are produced while a piece of work is in progress. They are
temporary. Once their content has been absorbed into the durable layer, they are
deleted from the working tree. Nothing is lost: version control retains them.

**Durable records** are the accumulated assets of the project. They are the only
documents anyone — human or agent — should need to read in order to understand
the system as it stands today and why it stands that way.

Every rule below exists to keep this separation honest.

A third kind of document sits alongside these two layers: a **Retrospective** is
durable — kept rather than deleted, like the records above — but it is opinion,
not record. It does not describe the system and nothing is required to agree
with it or act on it. See its entry below for what that distinction means in
practice.

---

## Document Types

### Work Log — temporary, append-oriented

A Work Log covers one unit of work, from the original request to its verified
completion. It records how the request was interpreted, what was planned, what
was actually done, what was discovered along the way, and how the result was
checked.

It is append-oriented. Earlier plans, wrong turns, and superseded findings stay
visible rather than being rewritten to match the final result. The value of a
Work Log while work is in flight is precisely that it shows the path, not just
the destination.

The Request section at the top of the Work Log states what was asked, written in
English as a restatement of the intent rather than a transcription of the
instruction. Instructions arrive in whatever language and phrasing the moment
allows; the log preserves their substance, not their surface. Restate the ask
faithfully — scope, constraints, and stated reasons all survive — but do not
carry over wording, typos, or slips that add nothing.

Earlier entries in the Request section are not rewritten. Later clarifications
are appended to it as they arrive, each restated the same way, in order.
Everything the agent infers lives below the section, so the boundary between what
was asked and what was concluded is always visible.

A Work Log is deleted once the retirement checklist below is satisfied.

### Decision Record — durable, append-only

A Decision Record preserves a decision and the reasoning behind it.

Write one when a decision has durable consequences, when meaningful alternatives
were considered, when a real trade-off was accepted, or when reversing it would
be costly.

Also write one whenever a Work Log holds knowledge that is worth keeping but has
no home in a Design Document. Design Documents describe what the system is; they
cannot express what was tried and rejected, which approach looked promising and
failed, or which constraint is not obvious from the code. If such knowledge would
disappear when the Work Log is deleted, that is the signal to write a Decision
Record. This is the main safeguard that makes deleting Work Logs safe.

Decision Records are append-only. When a decision changes, write a new record;
never edit the reasoning of the old one. The single permitted edit to an existing
record is its Status line, and practice has settled on two distinct forms it
takes, not one:

- `superseded by DR-____` — nothing in the old record's Decision holds anymore;
  a reader arriving at it should treat it as history only.
- `narrowed by DR-____` — the old record's core reasoning still holds, and only
  a specific claim within it has been corrected or scoped down by the new
  record. Say which claim, either inline on the Status line when it is one
  sentence, or leave the detail to the new record's Consequences.

Marking a record superseded when most of it still holds erases reasoning a
future reader could otherwise still trust, so the choice between the two is not
a formality.

Records are numbered sequentially and never renumbered, since other documents
cite them by identifier.

### Design Document — durable, update-oriented

A Design Document describes the current intended state of some part of the
system. It is authoritative.

It is update-oriented: it is rewritten so that it always reads as a description
of the present. A reader must never have to reconstruct the current design by
replaying history. Where a design exists in a particular shape for a
non-obvious reason, the document states the current constraint plainly and cites
the Decision Record rather than retelling the story.

### Retrospective — durable, dated, non-authoritative

A Retrospective is a point-in-time opinion about how the project's own
decisions and its documentation have gone. It does not decide anything and it
does not describe the system; it comments on the accumulated Decision Records
and Design Documents from outside them, the way a reader arriving fresh might.

It is durable — kept, not deleted — but it carries no authority: it is not part
of what a reader needs in order to understand the system, and nothing else is
required to agree with it or act on it. It is written when useful, on no fixed
schedule.

It is dated and never rewritten. A later Retrospective may reach a different
opinion about the same area; that is a new file, not an edit to the old one.
Unlike a Decision Record, an old Retrospective is not marked superseded when
that happens — it was never a claim about the present, so it does not need
correcting, only reading with its date in mind.

---

## Language

Every document under `docs/` is written in English — Work Logs, Decision
Records, Design Documents, and Retrospectives alike, including titles, slugs,
and section bodies.

This holds regardless of the language a request arrives in. Conversation happens
in whatever language suits the people involved; the durable record is written in
one language so that anyone, human or agent, reads the same thing.

---

## Ownership

The distinction that matters is between appending and overwriting.

Append-oriented documents — Work Logs and new Decision Records — may be written
by an agent as work proceeds, without prior review. Appending cannot destroy
existing information, so the cost of a bad entry is low and correctable. A
Retrospective carries the same low cost for the same reason: it is always a new
file, never an edit to an old one.

Design Documents are overwritten by nature, and an overwrite can quietly erase
intent that no one remembers to restore. An agent may draft the update, but a
human confirms it before the work is considered complete. The same applies to
marking a Decision Record as superseded, since that changes how an existing
record is read.

---

## Reading Order

The durable layer grows without bound, so the entry path must be explicit.

Begin with the Design Document index and read the documents covering the area
being changed. Consult Decision Records when a constraint seems arbitrary, when
an approach is about to be reversed, or when the rationale behind the current
shape matters to the task at hand. Reach for version control history, and any
archived Work Logs within it, only when the durable layer has genuinely failed to
answer the question — and treat that as a defect in the durable layer worth
fixing.

Do not load historical records speculatively. Context spent on history is context
unavailable for the work.

Skim `docs/retrospectives/` when planning new work, particularly the Try
sections of anything recent. They are opinions, not requirements, and nothing in
the durable layer cites them or depends on having read them — they exist to
change what gets planned next, not to be a source anything else points back to.

---

## Flow

```text
Human instruction
      │
      ▼
  Work Log  ──── significant decisions & preserved knowledge ──▶  Decision Records
      │                                                                  │
      └──────── resulting system state ──────────────────────────▶  Design Documents
                                                                         │
      ┌──────────────────────────────────────────────────────────────────┘
      ▼
 inputs to the next instruction
```

This is the typical shape, not a mandatory sequence. A decision is often made and
recorded before any implementation begins, and Design Documents are frequently
edited first when a change is specified before it is built. The durable layer
feeds the next request as much as it results from the last one.

Retrospectives sit outside this per-request flow. Rather than originating from
one instruction, they review the accumulated Decision Records and Design
Documents from outside them, on no fixed schedule, and their conclusions feed
forward into the Interpretation and Plan of future Work Logs rather than into
any single durable record.

---

## Retirement Checklist

A unit of work is complete, and its Work Log may be deleted, when all of the
following hold. Confirm them as the work finishes rather than at some later
cleanup pass; recording them at the end of the Work Log itself is the cheapest
way to do so.

- The resulting system state is reflected in the relevant Design Documents.
- Decisions with durable consequences are captured in Decision Records.
- Knowledge that cannot live in a Design Document — rejected alternatives,
  discovered pitfalls, non-obvious constraints — has a home in a Decision Record.
- No durable document depends on this Work Log, whether by citation or by
  implication.

Until these hold, the work is not finished, regardless of the state of the code.

---

## Layout and Naming

```text
docs/
  design/                       Design Documents
    index.md                    entry point; map of the durable layer
    <area>.md
  decisions/                    Decision Records
    DR-0001-<slug>.md
  retrospectives/               Retrospectives (durable, dated, non-authoritative)
    YYYY-MM-DD-<slug>.md
  work/                         Work Logs (temporary)
    YYYY-MM-DD-<slug>.md
```

One Work Log covers one request. If a request turns out to span several
independent pieces of work, split it into several Work Logs, each stating the
part of the request it answers, and cross-reference them.

A Retrospective's filename carries its date for the same reason a Work Log's
does: it is never rewritten, so a later opinion about the same area is a new
file rather than an edit to this one.

---

## Templates

### Work Log

```markdown
# <title>
Status: in progress | complete

## Request
<what was asked, restated in English: the intent, scope, and any stated reason>

<later clarifications appended here, restated the same way, in order>

## Interpretation
What is being asked, what is out of scope, what was assumed.

## Plan
<superseded plans are struck through or marked, not deleted>

## Progress
Dated entries: what was done, what was found, what changed and why.

## Verification
How the result was checked.

## Retirement
- [ ] Design Documents updated
- [ ] Decision Records written (DR-____)
- [ ] Non-obvious knowledge preserved
- [ ] No durable document depends on this log
```

### Decision Record

```markdown
# DR-0001: <decision, stated as a sentence>
Status: accepted | superseded by DR-____ | narrowed by DR-____
Date: YYYY-MM-DD

## Context
The situation and the forces that made a decision necessary.

## Decision
What was decided.

## Alternatives
What else was considered, and why it was not chosen.

## Consequences
What this makes easy, what it makes hard, and what it would cost to reverse.
```

### Design Document

```markdown
# <area>
Updated: YYYY-MM-DD

## Purpose
What this part of the system is for.

## Structure
How it is arranged now.

## Interfaces
What it exposes, and what it depends on.

## Constraints
Conditions that must hold, each citing its Decision Record where one exists.
```

### Retrospective

```markdown
# <title>
Date: YYYY-MM-DD

## Scope
What was reviewed, and as of when.

## Keep
What is working, and why it is worth continuing.

## Problem
What is not working, with evidence.

## Try
What to change next, and the trade-off it accepts.
```

---

## Core Principle

Work Logs preserve how the work unfolded, and are discarded once their value has
been extracted.

Decision Records preserve why the system is the way it is, and why it is not
some other way.

Design Documents preserve what the system is intended to be now.

Retrospectives preserve an opinion about how well the first three are working,
dated so it is read as what was true then, not asserted as what is true now.
