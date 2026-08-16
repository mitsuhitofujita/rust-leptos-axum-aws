# DR-0023: AWS behaviour is not re-implemented locally

Status: narrowed by DR-0028
Date: 2026-08-13

## Context

DR-0021 decided that the deployed edge is reproduced locally, outside the
service, and `crates/devgateway` reproduces five of its behaviours: the route
table, the preflight answered ahead of the authorizer, the 401 for a request
with no token, the `x-amzn-request-context` header, and the stringification of
claims. DR-0022 added a sixth, the authorizer's actual verdict against a real
token.

Nothing about that rig is broken. What has changed is the judgement of what it
costs.

Both records named the cost themselves, and accepted it. DR-0021's Consequences:
the route table in `crates/devgateway/src/edge.rs` is a hand-maintained copy of
`local.api_methods` in `infra/api/apigateway.tf`, and "a drift between the two is
visible the first time the stand-in is used and invisible until then." DR-0022's:
the audience rule in `check_audience` "is a hand-maintained copy of API Gateway's
behaviour", and "if AWS changes how a JWT authorizer resolves an audience,
nothing here will notice."

Those two admissions are the same admission, and the second is worse than the
first. A copy of `apigateway.tf` is a copy of something in this repository, which
someone editing that file might plausibly remember. A copy of API Gateway's
behaviour is a copy of a specification held by AWS, published in prose, changed
without reference to this project, and observable only by running against the
real thing — which is the thing the copy exists to avoid running against.

The deeper problem is that the set has no boundary. Five behaviours were
reproduced because five were noticed. Nothing distinguishes them from the ones
that were not noticed, and nothing says when the set is complete. Every
additional fidelity is more of AWS's specification written down a second time, in
Rust, in a repository that cannot test it against the original.

## Decision

**AWS behaviour is not re-implemented locally.** The application proper is
separated from AWS at its own edge, and verification that genuinely requires real
AWS is performed against real AWS.

Concretely:

- The Rust web application runs as an ordinary HTTP server, under `cargo run`.
- Inside the application, AWS-specific credential material is not handled
  directly. It is converted into a common `AuthContext` first — DR-0024.
- Local development has two authentication arrangements, chosen by what is being
  worked on: mock authentication for ordinary work, and local verification of a
  real Cognito token when authentication itself is the subject — DR-0022.
- Google sign-in, and Cognito itself, are used from real AWS.
- The behaviour of API Gateway, the Cognito authorizer and the Lambda Web Adapter
  *in combination* is verified against real AWS, not locally.
- SAM and local Lambda execution, if either is ever used, are bounded to Lambda
  packaging and to checking that the binary answers HTTP. Nothing about the edge
  is verified that way.
- `crates/devgateway` is reduced to a thin adapter that converts a verified JWT
  into an `AuthContext`, and does nothing else.

### The line this draws

Not everything local is a re-implementation, and the distinction is what decides
each case.

**Running AWS's own artefact locally is not re-implementation.** DynamoDB Local
is AWS's own binary, answering a real `Query` against a real key encoding
(DR-0020). Fetching the pool's key set from `{issuer}/.well-known/jwks.json` and
verifying a real token against it uses the real pool's real keys, and the token
is one Cognito actually issued (DR-0022). Neither of these teaches anything
false, because neither is a second telling of what AWS does — the first *is* what
AWS does, and the second reads what AWS published.

**Writing down what AWS does, so that it can be observed without AWS, is
re-implementation.** A route table transcribed from `local.api_methods`, a
preflight answered the way an HTTP API answers one, a 404 rather than a 405 for
an unrouted method, claims rendered `[a b]` because payload format 2.0 renders
them that way — each of these is a sentence from AWS's documentation, restated as
code, in a place where nothing can check the restatement.

`check_audience` sits on the line and is kept, deliberately. It is a
re-implementation by this test, and DR-0022 says so. It is retained because what
it guards — `jwt_configuration` in `infra/api/apigateway.tf` being wrong in a way
that surfaces only as an indistinguishable 401 after an apply — is a
configuration fault this repository owns, and because the rest of that mode is on
the artefact side of the line. The exception is named here so that it is
understood as one, rather than as a precedent.

### What is carried forward from DR-0021

One principle, and it survives intact: **the adapter is outside
`crates/server`.** DR-0021's argument holds and is not weakened by anything here
— a stand-in compiled into the service would destroy the property it exists to
check, and the reduced adapter stays a separate crate for exactly that reason.

Everything else in DR-0021 is retracted, which is why its Status line points
here.

## Alternatives

**Keep the rig and keep extending it.** The direction the work was already
travelling in, and the one this decision stops. Rejected because the maintenance
has no terminus: the obligation is to track a specification this project does not
hold, using a mechanism — someone happening to run the stand-in — that reports
drift late or not at all. The rig's fidelity would have had to grow to stay
useful, and each increment buys a smaller reduction in the risk it addresses.

**Keep the rig but freeze it.** Reproduce nothing further, and treat the five
behaviours as a fixed set. Rejected because it does not remove the copies, only
the growth: the route table still has to follow `local.api_methods`, and a frozen
mirror is worse than a growing one, since a reader would reasonably assume it
reflects the edge as it stands.

**LocalStack, or the AWS SAM CLI.** Rejected by DR-0021 already, and the grounds
are unchanged: both emulate far more of AWS than is wanted, both are a second
toolchain to pin in the devcontainer image, and both are Python, which this
container does not have and `CLAUDE.md` records as a deliberate absence. This
decision adds a further ground that applies to the whole category: an emulator is
someone else's re-implementation of AWS, which trades a copy this project
maintains for a copy it does not control at all. **The mention of SAM in the
instruction that produced this record is a boundary, not an adoption** — it
states how far SAM would be allowed to go if it were ever introduced, and it is
not being introduced.

**Verify tokens inside `crates/server`.** A separate question, refused by
DR-0017 and settled again by DR-0024. Not re-argued here.

## Consequences

**The route-table drift disappears.** `crates/devgateway` no longer has a route
table, so a method added to `local.api_methods` has one place to live again. This
is a return on the decision rather than an incidental tidy-up: it removes one of
the two hand-maintained copies named in the Context, and `workspace.md` loses the
constraint that described it.

**Three terminals become two.** `dev-api` and `dev-web`, as before DR-0021, with
the adapter joining only when a real token is the subject.

**`Bearer alice` and `Bearer bob` are gone**, and with them DR-0021's most useful
affordance. Two callers one `curl` flag apart is how the isolation
`identity::Owner` provides became checkable by hand. It is replaced rather than
lost: mock authentication gains a selectable subject, so the same check is made
without a stand-in at all — DR-0024.

**Some properties are no longer observable on a developer's machine.** A fourth
phase of the local-verification work was planned, to assert eight of them over
the rig with automated tests, and is cancelled by this decision. Its plan was the
only written inventory of what a deployment adds that a developer's machine does
not, so the inventory is preserved here, with where each one is checked now:

| Property | Checked where, now |
| --- | --- |
| Action types come back in creation order from a real `Query` | Local — DynamoDB Local (DR-0020) |
| The key encoding is `USER#…`/`TYPE#<ulid>` and `created_at` is 24 characters | Local — DynamoDB Local (DR-0015, DR-0020) |
| One owner cannot see another's action types | Local — mock authentication with a selectable subject (DR-0024) |
| A request with no token is refused before the service is reached | Real AWS |
| A method not in `local.api_methods` is a 404 | Real AWS |
| A preflight is answered without a token | Real AWS |
| A non-string claim degrades to the development owner | Neither — removed structurally by DR-0024 rather than tested around |
| A forged `x-amzn-request-context` does not reach the service | Real AWS — the property belongs to API Gateway, and only API Gateway can demonstrate it |

The seventh row is the one that changes character rather than location. DR-0021
was built partly to expose that silent misattribution; retracting the rig would
have left it unobserved, so DR-0024 removes the failure mode instead of arranging
to watch for it.

**A class of fault now surfaces after an apply rather than before one.** A method
missing from `local.api_methods`, a preflight blocked by an `ANY` route, a
`{proxy+}` path that does not match — each of these was visible locally under
DR-0021 and is not any more. This is the cost of the decision, accepted
knowingly: those faults are cheap to diagnose against real AWS, where the answer
is authoritative, and expensive to keep predictable locally, where the answer is
a guess this repository maintains. `just tf-validate` still schema-checks all
five layers before an apply.

**Reversing this costs more than it saved.** `crates/devgateway`'s `local` and
`passthrough` modes, its route table, its preflight and its context builder are
deleted by the work this decision authorises. Restoring them means writing them
again, against whatever AWS's specification says at that time — which is
precisely the property that made them expensive.
