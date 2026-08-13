# The local development policy

Status: in progress
Started: 2026-08-13
Branch: main

## Request

Settle how local development works, and stop the direction the local rig has been
travelling in.

No local Gateway that fully reproduces AWS behaviour is to be built. Emulating
API Gateway and the Cognito authorizer means implementing AWS's specification a
second time, in this repository, and that second implementation has to be
maintained against a specification nobody here controls.

The policy is:

- The Rust web application runs as an ordinary HTTP server, under `cargo run`.
- Inside the application, AWS-specific credential material is not handled
  directly. It is converted into a common `AuthContext` first.
- Local development uses two authentication arrangements, chosen by what is
  being done: mock authentication for ordinary work, and local verification of a
  real Cognito JWT when authentication itself is what is being checked.
- Google sign-in, and Cognito itself, are used from a real AWS environment
  provisioned for development.
- The precise behaviour of API Gateway + the Cognito authorizer + the Lambda Web
  Adapter in combination is verified in that development AWS environment, not
  locally.
- SAM and local Lambda execution are bounded to Lambda packaging and to checking
  that the binary answers HTTP.
- The existing Gateway is reduced, if it is needed at all, to a thin adapter that
  does no more than convert a JWT into an `AuthContext`.

The reasoning underneath all of it: do not re-implement AWS locally. Separate the
application proper from AWS, and perform in the cloud only the verification that
genuinely requires real AWS.

### Clarifications

This Work Log records the policy and settles its consequences for the documents
and the open plans. Introducing `AuthContext` and reducing the Gateway are
separate pieces of work, started once the policy is fixed.

The Gateway is to be reduced to the thin adapter — JWT verification producing an
`AuthContext`, and nothing else. The route table, the preflight handling, the
404-for-an-unrouted-method behaviour and the stringification of claims are not
kept.

The fourth phase of the earlier local-verification work,
`docs/work/2026-08-11-end-to-end-verification.md`, is cancelled. Its Work Log is
retired, with the reason recorded.

The sentence about SAM is a statement of a boundary, not an instruction to adopt
SAM. It says how far SAM would be allowed to go if it were ever used. The
existing constraint against Python in this container stands, and SAM is not being
introduced.

## Interpretation

**What is being asked.** A direction is being reversed. DR-0021 decided that the
deployed edge is reproduced locally, outside the service, and `crates/devgateway`
reproduces five of its behaviours. This request judges that reproduction to be
the wrong trade: the copies are copies of a specification held by AWS, and
keeping them honest is unbounded work with no natural stopping point.

What replaces it is a boundary rather than a rig. The application is made
independent of AWS at its own edge — an `AuthContext` it defines, rather than
API Gateway's request context read directly — and everything about how AWS
produces that context is verified where it actually runs.

The immediate deliverable is the durable layer catching up: a Decision Record
recording the policy and what it retracts, the Design Documents describing the
arrangement the policy asks for, and the open plans that contradict it either
brought into line or closed.

**What survives, and what does not.**

| Existing decision | Under this policy |
| --- | --- |
| DR-0017 — the service reads its caller from the adapter's request context | Changes shape. The service reads an `AuthContext`; something in front produces it. The security argument is unaffected |
| DR-0018 — the service runs without AWS, on an in-memory store and a development owner | Survives, and is what "mock authentication" names |
| DR-0020 — local verification runs against DynamoDB Local | Untouched. The table is not AWS behaviour being re-implemented; it is AWS's own binary being run |
| DR-0021 — the deployed edge is reproduced locally, outside the service | Largely retracted. "Outside the service" survives; "reproduces the five behaviours" does not |
| DR-0022 — real Cognito tokens are verified locally by the stand-in | Survives, and is the second of the two local arrangements the policy names |

The distinction the policy draws is between running AWS's own artefact locally
and re-writing what AWS does. DynamoDB Local is the first. A hand-written route
table copied from `apigateway.tf` is the second. JWKS verification sits with the
first in spirit — the key set is fetched from the real pool and the token is a
real token — which is why DR-0022 is not disturbed by a policy that retracts
DR-0021.

**Out of scope.**

- Implementing `AuthContext` in `crates/server` and `crates/app`. Decided here,
  built in a later Work Log.
- Reducing `crates/devgateway`. Same.
- Provisioning the development AWS environment. The policy names it; standing it
  up is its own work, and Cognito and the pool already exist.
- Adopting SAM, or any other Lambda emulator. The request bounds SAM; it does not
  ask for it.
- `docs/work/2026-08-10-api-artefact-packaging.md`. The packaging decision there
  — a container image built in multiple stages — is unaffected by this policy and
  is not reopened.
- `docs/work/2026-08-11-local-token-verification.md`. It is open pending four
  checks against a real pool. The policy makes those checks easier to run rather
  than unnecessary, so that log is left as it stands.

**Assumptions.**

- The `AuthContext` conversion happens outside `crates/server`, in the reduced
  adapter, and the service reads the converted value. This follows from the
  request keeping an adapter at all; if the service were to verify the JWT
  itself, no adapter would be needed and DR-0017 would be reversed rather than
  reshaped.
- "Mock authentication" is what `crates/server` already does under `just dev-api`
  — a constant development owner when nothing identifies the caller (DR-0018) —
  expressed in terms of `AuthContext` rather than of a missing header. The
  request asks for a mode, and the mode exists.
- The reduced adapter keeps `cognito` mode's verification whole. The policy
  removes the parts that imitate AWS, and verifying a real token against the real
  pool's real key set is not one of them.
- Cancelling the fourth phase does not mean the properties it listed go
  unchecked. The ones that belong to the store and the owner remain checkable
  locally; the ones that belong to the edge move to the development AWS
  environment, which is exactly the split the policy draws.

**A conflict worth naming.** DR-0021's Alternatives section rejected SAM partly
because this container has no Python and deliberately does not want it, and the
project instructions in `CLAUDE.md` say the same. The request's SAM sentence has
been read as a boundary rather than an adoption, per the clarification above, so
there is no conflict left to resolve — but the boundary is worth writing into the
Decision Record, because the next reader of that sentence will otherwise ask the
same question.

## Plan

1. **Write the Decision Record for the policy.** What is decided, what it
   retracts from DR-0021, why re-implementing a specification held by AWS was
   judged the wrong cost, and the line it draws between running AWS's own
   artefact locally and re-writing AWS's behaviour. It carries the rejected
   alternative explicitly — that the rig could have been kept and extended — and
   the SAM boundary.

2. **Mark DR-0021 superseded.** Its Status line points at the new record. This is
   the one permitted edit to an accepted record and needs confirmation before it
   is made.

3. **Decide whether the `AuthContext` boundary is its own Decision Record**, or a
   part of the policy record. It reshapes DR-0017, which argues for its own; it
   follows directly from the policy, which argues against. Proposed as its own,
   so that DR-0017's Status line has something specific to point at.

4. **Cancel the fourth phase.** Append the cancellation and its reason to
   `docs/work/2026-08-11-end-to-end-verification.md`, then retire it — the
   properties it was going to pin, and where each of them goes now, have to
   survive in a Decision Record first, because that log is the only place they
   are written down.

5. **Draft the Design Document updates**, for confirmation:
   - `backend.md` — the service reads an `AuthContext`; the header and its
     parsing move out.
   - `workspace.md` — the recipes, and what `crates/devgateway` is once it is
     reduced.
   - `deployment.md` — that the edge's combined behaviour is verified in the
     development AWS environment, and what that means for what an apply is
     expected to reveal.
   - `index.md` — the record table.

6. **Record the follow-on work** so it is not lost when this log retires: the
   `AuthContext` introduction, and the reduction of `crates/devgateway`. Each
   becomes its own Work Log once the policy is confirmed.

## Progress

### 2026-08-13

Log opened. Read `docs/README.md`, `docs/design/index.md`, `backend.md`,
DR-0021, DR-0022, `crates/server/src/identity.rs`, and the three open Work Logs
before writing anything.

**Two decisions warrant Decision Records**, and are noted here because this log
is temporary: the policy itself, which retracts most of DR-0021 seventeen days
after it was accepted, and the `AuthContext` boundary, which reshapes DR-0017.
Neither can live in a Design Document — a Design Document says what the system
is, not what was tried and judged too costly to keep.

**Knowledge in DR-0021 and in the fourth phase's log that must survive**, since
one is about to be superseded and the other deleted:

- The route table in `crates/devgateway/src/edge.rs` and `local.api_methods` in
  `infra/api/apigateway.tf` are a hand-kept pair, which DR-0021 accepted as a
  cost. Retracting the copy removes the drift, and that is one of the concrete
  returns on this policy rather than an incidental tidy-up.
- The silent failure DR-0021 was built to expose is real and is not fixed by
  retracting the rig: `identity.rs` deserialises claims as
  `HashMap<String, String>`, and a single non-string claim makes the whole decode
  fail, `subject()` return `None`, and the request be attributed to the
  development owner instead of refused. Under this policy that is a property of
  the `AuthContext` conversion, and it needs a home in whichever record covers
  the boundary.
- The eight properties listed in the fourth phase's Interpretation are the only
  written inventory of what is unobservable outside AWS. They must be split —
  store and owner properties stay local, edge properties move to the development
  AWS environment — and the split recorded, or cancelling the phase discards the
  list along with the plan.

**Four questions settled before drafting.** The "Dev AWS environment" names the
single environment that already exists, reached from localhost through the one
app client — so `deployment.md`'s "There is one environment" stands and DR-0005
is untouched. A malformed auth context is refused while an absent one still means
the development owner. Mock authentication gains a selectable subject, which is
what replaces `Bearer alice`. And the SAM sentence is a boundary, not an
adoption.

**Written.** DR-0023 (the policy) and DR-0024 (the `AuthContext` boundary).
DR-0021's Status now points at DR-0023 and DR-0017's at DR-0024. Design Documents
updated: `backend.md`, `workspace.md`, `deployment.md`, one citation in
`frontend.md`, and `index.md`'s record table.

**A correction to the plan.** It said the fourth phase's cancellation entry and
the deletion of its log should be one commit. That is wrong: a file added and
removed in the same commit leaves its content in no tree, so the cancellation
would have survived only as a diff. It has to be two — commit the entry, then
retire the log. The commit was declined, so
`docs/work/2026-08-11-end-to-end-verification.md` is left in place carrying
`Status: cancelled` and its cancellation entry, and its deletion waits on that
first commit. Everything the deletion depends on is already done: the
eight-property inventory is in DR-0023's Consequences, and nothing durable cites
the file.

**Two Design Documents now describe a system slightly ahead of the code.**
`backend.md` says the service reads an `AuthContext` and `workspace.md` says
`crates/devgateway` is a thin adapter; neither is built. Both carry a dated note
at the top saying so. This is the Design Document's proper mode — it records the
current intended state — but the note matters, because a reader who opens
`identity.rs` expecting `AuthContext` should find out why from the document
rather than from the surprise.

## Verification

Nothing to verify by execution: this log's product is documents. It is checked by
the durable layer being consistent afterwards, and each of these was run.

- **No Design Document describes the retracted rig.** `rg 'DR-0021' docs/design/`
  returns four hits, all of them deliberate: three sentences saying what was
  retracted and why, and the `index.md` row marking it superseded.
  `rg 'Bearer alice|passthrough|dev-gateway-cognito|DEVGATEWAY_MODE'` over
  `docs/design/` returns nothing but the dated note in `workspace.md` recording
  that the crate has not been reduced yet.
- **Every superseded record says so.** DR-0017 → DR-0024, DR-0021 → DR-0023.
  DR-0018, DR-0020 and DR-0022 still read `accepted`, which is correct: the
  policy leaves all three standing.
- **`index.md`'s record table lists exactly the files in `docs/decisions/`**,
  checked by diffing the two lists rather than by reading them.
- **Nothing durable cites a Work Log.** `rg 'end-to-end-verification'` over
  `docs/design/` and `docs/decisions/` returns nothing; an early draft of DR-0023
  named the file by path, which would have been a dangling pointer once it is
  deleted, and that was reworded.

## Retirement

- [x] Design Documents updated — `backend.md`, `workspace.md`, `deployment.md`,
      `frontend.md`, `index.md`. Drafted and applied; awaiting confirmation,
      which `docs/README.md` requires before the work counts as complete
- [x] Decision Records written — DR-0023 for the policy, DR-0024 for the
      `AuthContext` boundary; DR-0021 and DR-0017 Status lines updated
- [x] Non-obvious knowledge preserved — why reproducing AWS behaviour was judged
      the wrong cost, and that both DR-0021 and DR-0022 had already admitted the
      cost themselves (DR-0023 Context); the line between running AWS's artefact
      and re-writing its behaviour, with `check_audience` named as a deliberate
      exception rather than a precedent (DR-0023 Decision); the eight-property
      inventory and where each is checked now (DR-0023 Consequences); the
      `HashMap<String, String>` silent misattribution and why it is removed
      structurally rather than tested around (DR-0024 Context); the SAM boundary
      (DR-0023 Alternatives)
- [ ] No durable document depends on this log
- [ ] The fourth phase's log retired — its cancellation entry is written and its
      knowledge is in DR-0023, but the file cannot be deleted until that entry is
      committed, or the reasoning survives only as a diff
- [ ] Follow-on work opened or explicitly deferred — introducing `AuthContext`
      across `crates/server` and the adapter, and reducing `crates/devgateway`.
      Both are decided here and neither is built; two Work Logs, and this one
      cannot retire before they exist, because the dated notes in `backend.md`
      and `workspace.md` are the only record that the code has not caught up
