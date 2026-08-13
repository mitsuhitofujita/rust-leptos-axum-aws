# Settling the local development policy

## Context

`crates/devgateway` was built over two Work Logs in August to reproduce the
deployed edge on a developer's machine: a route table copied from
`local.api_methods` in `infra/api/apigateway.tf`, an `OPTIONS` answer, a 404 for
an unrouted method, claims stringified the way API Gateway stringifies them, and
a JWT authorizer. DR-0021 accepted the hand-maintained copies as the price;
DR-0022 added real token verification on top.

The user has judged that trade wrong. The copies are copies of a specification
AWS holds and can change, the set of behaviours worth copying has no natural
boundary, and drift is invisible until someone happens to run the rig. The new
policy: **do not re-implement AWS locally.** Separate the application from AWS at
its own edge — a common `AuthContext` rather than API Gateway's request context
read directly — run the app as an ordinary HTTP server, and perform in real AWS
only the verification that genuinely needs real AWS.

The line this draws is between *running AWS's own artefact locally* and
*re-writing what AWS does*. DynamoDB Local is the first, and survives untouched.
JWKS verification against the real pool's real keys is also the first, and
survives. The route table, the preflight and the claim stringification are the
second, and go.

**This plan produces documents only.** The `AuthContext` introduction and the
reduction of `crates/devgateway` are decided here and built in later Work Logs.
The working record is `docs/work/2026-08-13-local-development-policy.md`.

## Decisions already settled with the user

- Scope is the durable layer plus the disposal of contradicting plans. No code.
- `crates/devgateway` reduces to JWT verification producing an `AuthContext`.
  The route table, preflight, 404 behaviour, claim stringification, and the
  `local` and `passthrough` modes are not kept.
- `docs/work/2026-08-11-end-to-end-verification.md` is cancelled and retired.
- The SAM sentence is a boundary, not an adoption. Python stays out of the
  container; SAM is not introduced.
- "Dev AWS environment" names the single environment that already exists, used
  from localhost through the one app client. `deployment.md`'s "There is one
  environment" stands, and DR-0005 is untouched.
- Malformed auth context is **refused**; *absent* auth context still means the
  development owner. This preserves DR-0018's mock authentication while removing
  the silent misattribution.
- Mock authentication gains a selectable subject, so local owner-isolation checks
  survive the loss of `Bearer alice` / `Bearer bob`.

## Steps

### 1. `docs/decisions/DR-0023-aws-behaviour-is-not-reimplemented-locally.md`

The policy record. Load-bearing content:

- **Context** — what DR-0021 built and what has changed: not a defect in the rig
  but a re-judgement of its cost. Name the two hand-kept copies concretely
  (`edge.rs` route table vs `local.api_methods`; the `check_audience` rule vs API
  Gateway's behaviour) and that both were accepted knowingly in DR-0021's
  Consequences and DR-0022's.
- **Decision** — the seven policy points, plus the artefact/behaviour line above.
- **Alternatives** — keep and extend the rig (unbounded maintenance against a
  spec held elsewhere); LocalStack / SAM (rejected again, and now also on the
  Python constraint in `CLAUDE.md`, stated as the SAM boundary); verify inside
  `crates/server` (belongs to DR-0024, cross-reference rather than re-argue).
- **Consequences** — must carry the property split below, because
  `2026-08-11-end-to-end-verification.md` is the only place it is written down
  and that file is being deleted in step 4:

  | Property | Where it is checked now |
  | --- | --- |
  | Action types in creation order from a real `Query` | Local — DynamoDB Local (DR-0020) |
  | Key encoding `USER#…`/`TYPE#<ulid>`, `created_at` 24 chars | Local — DynamoDB Local |
  | One owner cannot see another's types | Local — mock auth with a selectable subject (DR-0024) |
  | A request with no token is refused before the service | Real AWS |
  | A method outside `local.api_methods` is a 404 | Real AWS |
  | A preflight is answered without a token | Real AWS |
  | A non-string claim degrades to the development owner | Removed structurally by DR-0024, not tested around |
  | A forged `x-amzn-request-context` does not reach the service | Real AWS — the property is API Gateway's |

  Also: three terminals become two; the route-table drift disappears, which is a
  return on this policy rather than an incidental tidy-up.

### 2. Mark DR-0021 superseded

One line: `Status: superseded by DR-0023`. The only permitted edit to an accepted
record, per `docs/README.md`. DR-0023 states explicitly which part it carries
forward — the adapter stays *outside* `crates/server` — so a reader arriving at
DR-0021 is not left thinking the whole thing was abandoned.

**DR-0022 is not superseded** and its Status line is untouched.

### 3. `docs/decisions/DR-0024-the-service-reads-an-authcontext.md`

- **Context** — DR-0017 has `crates/server/src/identity.rs` parse
  `x-amzn-request-context` and pull `sub` out of a `HashMap<String, String>`.
  That ties the service's identity code to API Gateway payload format 2.0, and
  the `HashMap<String, String>` is correct only because that format stringifies
  everything. One non-string claim makes the whole decode fail, `subject()`
  return `None`, and the request be attributed to the development owner rather
  than refused — a write into the wrong partition with no error anywhere.
  DR-0021 existed partly to expose this; the policy retracts that rig, so the
  answer has to be structural.
- **Decision** — the service defines `AuthContext` and reads only that. The
  AWS-specific conversion lives outside `crates/server`, in the reduced adapter,
  which is where DR-0021's one surviving principle applies. Both local
  arrangements produce the same `AuthContext`: mock authentication with a
  selectable subject, and real-token verification. Absent context → development
  owner (DR-0018 intact). Present but unreadable → refusal.
- **Deliberately not decided here**: the wire format between adapter and service
  (header name, encoding) and the extractor's exact shape. Those belong to the
  implementing Work Log.
- **Consequences** — DR-0017 gets `Status: superseded by DR-0024`; its security
  argument survives verbatim in DR-0024, since the adapter-to-service hop is
  still not a security boundary and still relies on nothing else being able to
  reach the service. `identity::Owner` stays the single extractor.

### 4. Cancel and retire the fourth phase

Append a dated cancellation entry to
`docs/work/2026-08-11-end-to-end-verification.md` — why it is cancelled, and
that its property inventory has moved into DR-0023's Consequences — then delete
the file. One commit, so the knowledge lands and the log leaves together.

`docs/work/2026-08-11-local-token-verification.md` stays open and unedited: it
awaits four checks against a real pool, and the policy makes those easier rather
than unnecessary. `docs/work/2026-08-10-api-artefact-packaging.md` is untouched.

### 5. Design Document drafts — each confirmed before it is written

Per `docs/README.md`, Design Documents are overwritten and a human confirms.
Present each diff, then apply.

- **`docs/design/backend.md`** — the Identity section rewritten around
  `AuthContext`; the header and its parsing leave the service. Three constraints
  reshaped: "the owner comes from the request context" → from the `AuthContext`;
  "the header is not a security boundary" → the adapter hop is not; "a missing
  header means development, not rejection" → split into absent vs malformed.
- **`docs/design/workspace.md`** — the largest edit. `crates/devgateway`'s
  description (l.42–48) becomes the thin adapter; the recipe table loses
  `dev-gateway` in its `local` sense; three constraints go — the route-table copy
  (l.159–165), "complementary, not ranked" (l.166–171), and the `Bearer alice`
  affordance in the prose at l.116–133. The `cognito` no-defaults constraint
  (l.172–177) stays. The `Trunk.toml` no-`[[proxy]]` constraint stays: there are
  still two backends.
- **`docs/design/deployment.md`** — "The edge is reproduced locally" (l.186–193)
  is replaced by a statement that the edge's combined behaviour is verified
  against real AWS. "The authorizer's configuration is checkable before an apply"
  (l.195–203) survives, reworded for the reduced adapter. The
  "since DR-0021, `crates/devgateway`'s route table as well" clause (l.181–182)
  goes. The `dev-gateway` 401 constraint (l.315–319) goes. The audience
  constraint (l.321–329) stays — it is DR-0022's.
- **`docs/design/frontend.md`** — one citation at l.205, the `dev-web-gateway`
  proxy backend. Re-point from DR-0021 to DR-0023.
- **`docs/design/index.md`** — add DR-0023 and DR-0024; annotate DR-0017 and
  DR-0021 as superseded, in the style DR-0012's row already uses.

### 6. Record the follow-on work

Two Work Logs are *not* opened here, and are named in this log's Retirement so
they are not lost when it is deleted: introducing `AuthContext` across
`crates/server` and the adapter, and reducing `crates/devgateway`.

## Verification

There is nothing to execute — the product is documents. It is checked by the
durable layer being self-consistent afterwards:

1. `rg -n 'DR-0021' docs/design/` returns nothing outside a superseded-record
   note; today it hits `deployment.md`, `workspace.md`, `frontend.md`,
   `index.md`.
2. `rg -n 'dev-gateway|devgateway|Bearer alice' docs/design/` describes only the
   reduced adapter — no route table, no preflight, no two-callers affordance.
3. DR-0017 and DR-0021 both carry a `superseded by` Status line pointing at a
   record that exists; DR-0022, DR-0018 and DR-0020 still read `accepted`.
4. `docs/design/index.md`'s record table lists every file in
   `docs/decisions/`, and no more.
5. `ls docs/work/` shows two logs: the packaging one and the token-verification
   one, plus this policy log until it retires.
6. No Design Document describes behaviour that `crates/devgateway` will no longer
   have once the follow-on work runs, and none describes an `AuthContext` shape
   that has not been decided.

Note that steps 5 and 6 leave the Design Documents describing a system slightly
ahead of the code: `crates/devgateway` still has its `local` mode and
`crates/server` still parses the header until the follow-on Work Logs run. That
gap is deliberate and is stated in the policy log, since Design Documents record
the current *intended* state.

## Files touched

| File | Action |
| --- | --- |
| `docs/decisions/DR-0023-aws-behaviour-is-not-reimplemented-locally.md` | create |
| `docs/decisions/DR-0024-the-service-reads-an-authcontext.md` | create |
| `docs/decisions/DR-0021-…md` | Status line only |
| `docs/decisions/DR-0017-…md` | Status line only |
| `docs/design/backend.md`, `workspace.md`, `deployment.md`, `frontend.md`, `index.md` | update, each confirmed |
| `docs/work/2026-08-11-end-to-end-verification.md` | cancellation entry, then delete |
| `docs/work/2026-08-13-local-development-policy.md` | Progress and Retirement as work proceeds |
