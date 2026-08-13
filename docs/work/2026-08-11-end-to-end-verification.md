# End-to-end verification against the local rig

Status: cancelled — see DR-0023
Started: 2026-08-11
Cancelled: 2026-08-13
Branch: main

## Request

Before the work in `docs/work/2026-08-10-api-artefact-packaging.md` is attempted,
establish a way to verify the system locally. The two parts of the deployed edge
that `crates/server` is written against but never exercises outside AWS are the
DynamoDB table and the authentication filter — API Gateway's JWT authorizer
together with the request context the Lambda Web Adapter forwards. Both are to be
reproduced on a developer's machine, following whatever is the best practice for
doing so.

The result is meant to be used for a long time, so it is to be built to last
rather than as a throwaway. Anything that has to be written is written in Rust:
this is a devcontainer and Python is not available. `ripgrep` is.

The four-phase shape proposed in conversation is accepted and all four phases are
to be carried out. The work may be split into Work Logs as seems best, ordered so
that each piece can be carried out on its own and the sequence can be abandoned
part way through, smallest first.

**This log answers the fourth phase: automated end-to-end tests over the rig.**
The others are `2026-08-11-local-dynamodb.md`, `2026-08-11-local-api-edge.md` and
`2026-08-11-local-token-verification.md`. This phase needs the first two and does
not need the third.

## Interpretation

**What is being asked.** The first three phases produce something a person can
run. This one makes it something that runs itself, so that the properties the
rig can now observe are asserted rather than remembered. Without it the rig
decays into three commands nobody types.

**What the tests are for.** One list, and it is not a general test suite for the
application. Every case is a property that is currently unobservable outside AWS,
and most are drawn from the consequences the durable layer already admits to:

| Property | Why it is here |
| --- | --- |
| Action types come back in creation order from a real `Query` | DR-0018 says the two stores agree only because the key embeds a ULID; nothing checks it |
| The key encoding is `USER#…` / `TYPE#<ulid>` and `created_at` is 24 characters | DR-0015; `store::TIMESTAMP` is the only thing enforcing it |
| One owner cannot see another's types | The single reason `identity::Owner` exists |
| A request with no token is refused before the service is reached | DR-0010 |
| A method not in `local.api_methods` is a 404 | DR-0009's trap |
| A preflight is answered without a token | DR-0009 |
| A non-string claim degrades to the development owner | The silent failure the second phase names |
| A forged `x-amzn-request-context` does not reach the service | DR-0017's argument, made visible |

**Out of scope.**

- The SPA. There is no browser and no Node.js in this container, and
  `workspace.md` already records that as a boundary. These tests drive HTTP.
- Anything requiring AWS. The tests run with no credentials and no network.
- Performance, cold starts, throttling, IAM. The rig cannot observe any of them
  and the tests must not pretend to.
- Making these part of `cargo test --workspace`. They need a JRE and built
  binaries, so a plain `cargo test` on a fresh clone must not start failing.

**Assumptions.**

- Spawning processes from a test is acceptable here. The alternative — importing
  the service as a library — would mean restructuring `crates/server` around
  testability, which is the coupling the whole approach avoids.
- Fixed non-default ports are enough to avoid colliding with a running
  development session; the tests do not need to be runnable concurrently with
  themselves.

## Plan

1. **`crates/e2e`**, a development-only crate whose tests spawn the three
   processes: DynamoDB Local, `server`, and `devgateway` in its `local` mode.
   Ports well away from 8000/3000/3001 so a development session can keep running.
2. **The harness** creates the table through the SDK rather than the CLI, so the
   tests are self-contained; the schema cites `docs/design/persistence.md` as the
   interface it copies, like the recipe in the first phase.
3. **Skip rather than fail when the JRE is absent**, with a message naming the
   recipe that installs it. A fresh clone on a machine without the devcontainer
   should report why, not produce a stack trace.
4. **The cases above**, one test each, each naming the record whose consequence
   it pins.
5. **`just test-e2e`**, which builds the two binaries first. `just test` is left
   alone.
6. **Documents.** Draft updates to `workspace.md` (the crate, the recipe, and why
   it is outside `just test`) and `backend.md` (that the store's DynamoDB half is
   now covered, which its current text says it is not), for confirmation. This
   phase probably needs no Decision Record of its own; that is judged when it is
   built.

## Progress

### 2026-08-13 — cancelled

Nothing was built. `crates/e2e` does not exist and no test was written.

This phase is cancelled by DR-0023, which decides that AWS behaviour is not
re-implemented locally. The plan above is built on `crates/devgateway`'s `local`
mode — step 1 spawns it as one of the three processes — and that mode is being
deleted. Four of the eight properties in the Interpretation table are properties
of the stand-in's reproduction of API Gateway rather than of this system, so
asserting them would have pinned the fidelity of a copy that DR-0023 retracts.

Cancelling is not the same as deciding the properties do not matter. They are
split, and the split is recorded in DR-0023's Consequences: the store and owner
properties stay locally checkable, the edge properties move to real AWS, and the
non-string-claim degradation is removed structurally by DR-0024 rather than
tested around.

**The part worth keeping was the table, not the plan.** The Interpretation
section above was the only written inventory of what a deployment adds that a
developer's machine does not. It has been copied into DR-0023 with a column
saying where each property is checked now, which is why this log can be deleted.

The reasoning behind step 3 — skip rather than fail when the JRE is absent, so a
fresh clone reports why instead of producing a stack trace — is worth reaching
for again if a local test harness is ever built for the DynamoDB half alone.
That is the one piece here with a future, and it is small enough to re-derive.

## Verification

Not run. Nothing was built to verify.

## Retirement

- [x] Design Documents updated — none needed. The drafts this log planned for
      `workspace.md` and `backend.md` described a crate that was never written
- [x] Decision Records written — DR-0023 cancels this phase and carries its
      property inventory; DR-0024 removes the seventh property's failure mode
- [x] Non-obvious knowledge preserved — the eight-property table is in DR-0023's
      Consequences with its split recorded
- [x] No durable document depends on this log — `docs/design/` and
      `docs/decisions/` were searched for this file's name and it does not
      appear. `2026-08-11-local-token-verification.md` cites it, but only in its
      Request section, which records what was asked and is not rewritten
