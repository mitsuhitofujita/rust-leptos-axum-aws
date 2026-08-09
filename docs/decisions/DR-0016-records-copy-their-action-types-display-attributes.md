# DR-0016: An action record copies its action type's display attributes

Status: accepted
Date: 2026-08-09

## Context

An action record is a numeric value paired with an action type, and every screen
that shows a record shows it through that type: the dashboard row draws the
type's icon, its name, the value, and the type's unit — `Running — 5.2 km`.

Action types are editable and deletable. `docs/design/page-layouts.md` defines
the edit and delete screens and states explicitly that the effect of a deletion
on existing records is unspecified. That gap has to be closed by the storage
design, because the two possible answers produce different items.

With the single-table key design of DR-0015, a record and its type are in the
same partition, so fetching both is one query rather than a cross-table lookup.
The question is therefore not about query cost. It is about which values a record
displays after its type has changed.

## Decision

**A record item stores `type_id` and also its own copy of the type's `name`,
`unit` and `icon`, as they were when the record was written.**

Displaying a record uses the copies. Nothing re-reads the type in order to
render history.

The identifier is kept alongside them, and it is the identifier — not the copied
name — that answers what the record is: two records of the same type remain the
same type after a rename, and the dashboard's "record this again" transition
resolves through `type_id`.

## Alternatives

**Store `type_id` alone and resolve the type at read time.** The normalised
answer, and cheap here, since the type is in the same partition. Rejected
because it makes a rename retroactive: correcting a type's unit from `km` to `m`
would silently restate every past record's magnitude, and renaming `Running` to
`Cycling` would rewrite history that never happened. A record is an observation
about a moment, and the labels it was recorded under are part of that
observation.

It also leaves deletion with no good answer. Deleting a type would either orphan
its records, which then have nothing to display, or force a cascade that
destroys history the user did not ask to lose.

**Store the copies and no `type_id`.** Rejected. It loses the grouping — the
dashboard row's "record this again" would have to match on a name, and two types
that were once identically named would merge.

**Never delete or rename a type; soft-delete instead.** Considered, because it
preserves history without duplicating anything. Rejected as the wrong burden:
the design gives the user a delete button and a confirmation dialog, and a type
that stays in the store forever still has to be hidden from every picker and
every list, which is complexity in every read path instead of four attributes on
a write.

## Consequences

Easy: a record renders from itself, with no second lookup and no join; deleting
an action type is a single-item delete that cannot damage history; and history
reads correctly years later, in the terms it was recorded in.

Hard, and accepted:

- **Editing a type does not change past records.** This is the intended
  behaviour, but it is a surprise the first time someone corrects a typo in a
  name and finds the old spelling still in the dashboard. If a screen ever needs
  to present it, that is a copy-writing problem, not a schema change.
- **A type's current values and a record's copies can disagree**, and nothing
  reconciles them. There is no consistency to maintain — they answer different
  questions — but a reader of the raw items has to know that.
- **A backfill is the only way to restate history**, should that ever be wanted:
  query the user's records, rewrite the copied attributes. Nothing supports this
  today.
- **Records are slightly larger.** Three short strings per item, which is
  irrelevant at any volume this project will see.
