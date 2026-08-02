---
name: work-log
description: Open a new Work Log for a unit of work, capturing the human instruction verbatim before any planning or implementation begins. Use when starting a new task, feature, bug fix, or refactor in this project.
argument-hint: "the instruction — or leave empty to use the previous message"
disable-model-invocation: true
allowed-tools: Bash(date *) Bash(git branch *) Bash(ls *) Read Write
---

# Open a Work Log

## Project state

- Today: !`date +%Y-%m-%d`
- Branch: !`git branch --show-current 2>/dev/null || echo "(not a git repository)"`
- Open Work Logs: !`ls -1 docs/work/ 2>/dev/null || echo "(none)"`
- Design Documents: !`ls -1 docs/design/ 2>/dev/null || echo "(none)"`
- Recent Decision Records: !`ls -1 docs/decisions/ 2>/dev/null | tail -5 || echo "(none)"`

## The request

$ARGUMENTS

## What to do

**Read `docs/README.md` first.** It defines how documentation works in this
project — the document types, what each is for, who may overwrite what, and the
retirement checklist that decides when a Work Log may be deleted. Read it before
creating anything. Where it and this skill differ, `docs/README.md` is
authoritative; say so rather than silently picking one.

**Write everything in English.** Work Logs, and every other document under
`docs/`, are written in English — title, slug, and all sections. Talk with the
user in whatever language they are using; the file itself is English.

**Establish the request.** If the section above is empty, the instruction is the
user's most recent message in this conversation instead. Either way, the Request
section of the log is a restatement in English of what was asked — the intent,
the scope, and any reason the user gave for it. Do not quote the instruction and
do not translate it clause by clause. Capture the substance faithfully; drop the
surface. Wording, typos, and slips in the original carry no information worth
preserving, and a log is read long after the phrasing stops mattering.

Restating is not interpreting. The Request section says what was asked; anything
you concluded, assumed, or ruled out goes in Interpretation, below it. That
boundary is the whole point of the section: a reader must be able to see the ask
separately from your reading of it. When in doubt about whether something was
asked or inferred, put it in Interpretation.

**Check for an existing log first.** If an open Work Log already covers this
request, append to it rather than creating a second one. If the request turns out
to contain several independent pieces of work, say so and propose a split before
creating anything — one Work Log covers one unit of work.

**Read the durable layer before writing.** Consult `docs/design/index.md` and the
Design Documents covering the affected area. Read a Decision Record when the
request would reverse or bump into something already decided. Do not read old
Work Logs unless the durable documents genuinely fail to answer a question.

**Create the file** at `docs/work/YYYY-MM-DD-<slug>.md`, using today's date above
and a short hyphenated slug describing the work. Follow the template exactly.

**Write Interpretation carefully.** State what is being asked, what is explicitly
out of scope, and every assumption you are making. Assumptions belong here even
when they seem obvious — this section is where a misread gets caught cheaply.

**Ask about anything genuinely ambiguous.** Then append what the answers
established to the Request section, in English, under a `### Clarifications`
heading, in the order they arrived. Do not fold them back into the original
restatement — a clarification that changed the scope should be visible as a
change.

**Stop after the Interpretation and Plan are written.** Present them and wait for
confirmation before implementing. The log exists so that the interpretation can
be checked before effort is spent on it, not after.

## Template

```markdown
# <title>

Status: in progress
Started: YYYY-MM-DD
Branch: <branch>

## Request

<what was asked, restated in English: the intent, the scope, any stated reason>

## Interpretation

What is being asked. What is out of scope. What is assumed.

## Plan

<numbered steps>

## Progress

### YYYY-MM-DD
<what was done, what was found, what changed and why>

## Verification

How the result was checked.

## Retirement

- [ ] Design Documents updated
- [ ] Decision Records written (DR-____)
- [ ] Non-obvious knowledge preserved — rejected alternatives, pitfalls, constraints
- [ ] No durable document depends on this log
```

## Standing instructions for the rest of this session

These apply to every later turn of this work, not just to creating the file.

Everything written into the log stays in English, including entries added later.

Append to Progress as work proceeds, dated. When a plan turns out to be wrong,
mark the old step as superseded and write the new one below it. Do not edit an
earlier entry to match what actually happened — the record of the wrong turn is
often the most valuable thing in the log.

When a decision surfaces that has durable consequences, involves real
alternatives, or would be costly to reverse, note it in Progress and tell the
user it warrants a Decision Record. The same applies to anything learned that
cannot live in a Design Document: an approach that was tried and failed, a
constraint that is not obvious from the code. This log will be deleted once the
work is done, so knowledge with no durable home is knowledge about to be lost.

You may write and append to this log freely. Updating a Design Document
overwrites existing content, so draft the change and have the user confirm it
before marking the work complete.
