# Select the IaC tool and the configuration layering for the AWS infrastructure

Status: in progress
Started: 2026-08-02
Branch: main

## Request

Choose the Infrastructure as Code tool used to create this project's AWS
infrastructure. Terraform is the leading candidate and should be evaluated
first, but the choice is to be reasoned about rather than assumed.

The configuration is to be divided by the risk profile of the resources it
manages — layers along the lines of logging, database, network — rather than by
deployment environment. Splitting the configuration into a `prod` set and a
`dev` set is explicitly not what is wanted.

The infrastructure the choice has to cover is the stack already selected for the
project:

| Concern | Choice |
| --- | --- |
| Frontend | Rust + Leptos, CSR / SPA |
| Frontend hosting | Amazon S3 + Amazon CloudFront |
| Authentication | Amazon Cognito User Pools, with Google as an identity provider |
| Authentication flow | Authorization Code Flow with PKCE |
| API | Rust + axum |
| API runtime | AWS Lambda with the AWS Lambda Web Adapter |
| API exposure | Amazon API Gateway HTTP API |
| API authorization | API Gateway JWT authorizer |
| Project management | Cargo workspace |

Most of this stack was already fixed by DR-0001; the runtime and authentication
choices above are new and arrive with this request.

### Clarifications

Asked on 2026-08-02, before any evaluation was done.

- **There is one environment.** No `dev` deployment exists on AWS. Development
  happens locally, against `trunk serve` and `cargo run -p server`. The risk
  layering is therefore the only axis along which the configuration is divided,
  not one axis of two.
- **The layers are separate state files.** Each layer is an independent root
  module with its own `tfstate` and its own apply, so that a mistaken destroy is
  contained within one layer. An upper layer reads a lower layer's outputs;
  which mechanism carries them is left to the design.
- **The deliverable is the decision and the design, not the code.** The work
  ends with the Decision Records and `docs/design/deployment.md` written. No
  `.tf` files are produced by this unit of work.

## Interpretation

**What is being asked.** Two decisions, and the reasoning that supports them:

1. Which IaC tool this project uses, with Terraform evaluated first and the
   realistic alternatives named and dismissed on their merits rather than
   ignored.
2. How the configuration is decomposed — what the layers are, where the state
   boundaries fall, and how a layer consumes what a lower layer produced.

**What is out of scope.** Writing the Terraform itself, the CI/CD pipeline that
runs it, and any actual AWS account or resource creation. This unit of work ends
with the decision recorded and the deployment design written down. Also out of
scope: revisiting the application-stack choices in the table above — they are
inputs here, not questions.

**What is assumed.**

- "Split by risk, not by environment" describes how root modules and state files
  are divided. Originally it was unclear whether a `dev` deployment existed at
  all; the clarification above settles that it does not, which removes
  Terraform workspaces and per-environment variable files from consideration
  entirely rather than merely deprioritising them.
- One environment plus a hosted identity provider has a consequence the design
  has to absorb: the Cognito app client is the same one local development
  authenticates against, so its callback URLs must include the local dev server
  alongside the CloudFront domain. This is a property of the layer that owns
  Cognito, not a separate dev environment sneaking back in.
- `log > db > network` is illustrative of the axis — category and blast radius —
  rather than a literal list to implement. The current stack has no database at
  all, and the layer set has to be derived from the resources this project
  actually has.
- Statefulness is the practical proxy for the risk being layered against: a
  Cognito User Pool holds user identities and cannot be recreated without losing
  them, so it sits on the low-churn side of the split even though it is not a
  database.
- AWS is the only target. Multi-cloud portability carries no weight as a
  selection criterion.
- "Terraform" means HashiCorp Terraform under the BUSL licence; OpenTofu is a
  distinct candidate and is evaluated as one.
- The frontend build already forbids Node.js anywhere in the toolchain
  (`docs/design/frontend.md`). Any IaC tool whose authoring language is
  TypeScript inherits that constraint as a cost.

## Plan

1. ~~Resolve the open questions below with the user, and append what they settle
   to the Request section.~~ Done 2026-08-02; see Clarifications.
2. Evaluate the candidates — Terraform, OpenTofu, AWS CDK, AWS SAM,
   CloudFormation, Pulumi, SST — against criteria this project actually imposes:
   coverage of Cognito, CloudFront and API Gateway; support for splitting state
   by layer; the no-Node.js constraint; how the Lambda artefact and the `dist/`
   upload are handled; and licensing.
3. Design the layer decomposition: the layers themselves, the state boundary of
   each, the direction of dependencies between them, and the mechanism by which
   an upper layer reads a lower layer's outputs. Include the bootstrap problem —
   the state backend itself has to be created by something, and it is the one
   piece of infrastructure no layer can depend on having.
4. Decide how application artefacts — the WASM bundle in `dist/` and the Lambda
   package — relate to the infrastructure layers, since these deploy on a
   different cadence from everything else.
5. Write the Decision Records: DR-0004 for the tool, DR-0005 for the layering.
   Merge them into one record only if they turn out to be a single decision.
6. Draft `docs/design/deployment.md` and update `docs/design/index.md`, which
   currently lists deployment as not yet written. Both need confirmation before
   the work is marked complete.

## Progress

### 2026-08-02

Work Log opened. Read `docs/README.md`, the Design Document index,
`workspace.md`, `frontend.md`, and DR-0001.

Three constraints already recorded in the durable layer bear directly on this
work, and the deployment design has to answer all of them:

- DR-0001 records that the API needs CORS in production and that the absence of
  a CORS layer in `crates/server` is a gap deployment must close, not a
  decision.
- DR-0001 records that CloudFront needs a custom error response mapping 403/404
  to `/index.html` with status 200, or every reload on a non-root route fails.
- `docs/design/index.md` names deployment as a document that does not exist yet
  and lists exactly this scope: S3, CloudFront, the API's runtime, IaC, CI/CD.

Questions raised with the user before evaluating anything; the answers are
recorded under Clarifications. All three narrowed the work: one environment, one
state file per layer, documentation as the deliverable.

Interpretation and Plan presented for confirmation. No evaluation done yet.

Plan confirmed. Steps 2–6 carried out.

**Step 2 — the tool.** The candidate field collapsed faster than expected, and
on constraints this project had already recorded rather than on any general
comparison. CDK, CDKTF and SST all require Node.js for the CLI regardless of
authoring language, which `frontend.md` rules out. SAM covers the serverless
half well and the CloudFront/Cognito/S3 half badly, so it would have meant two
tools. Raw CloudFormation's cross-stack exports cannot be changed while another
stack imports them, which freezes exactly the layer boundaries this work wants
to keep movable. Pulumi has no official Rust SDK, so it would not have unified
the language even if adopted.

That left Terraform against OpenTofu, which is a genuine tie on capability —
same provider, drop-in. Terraform was chosen on ecosystem gravity alone, so the
decision was recorded together with the restraint that follows from it: no
Terraform-only features, keeping the switch cheap. Recorded as DR-0004.

**Step 3 — the layering.** Four layers: `bootstrap`, `delivery`, `identity`,
`api`. Two findings shaped the set.

First, the dependency order is not the risk order. Risk says Cognito is the most
dangerous thing here; dependency says `delivery` must be applied before
`identity`, because the Cognito callback URLs need the CloudFront domain and
nothing about CloudFront needs Cognito. The chain
`delivery → identity → api` is forced and has no cycle, but it does mean the
apply order and the "how bad is destroying this" order are different lists, and
only the first one can be followed mechanically.

Second, `identity` and `delivery` were nearly merged — both are low-churn,
both hold something that is painful to lose. Kept apart because losing a
CloudFront distribution costs an outage and a domain change, while losing a user
pool costs the users, and collapsing those into one blast radius defeats the
whole exercise.

Cross-layer values go through SSM Parameter Store rather than
`terraform_remote_state`. The deciding argument was not access scope, though
that is real — it was that the SPA build needs the same values, so one
publication mechanism serves both consumers instead of the build needing its own.

Bootstrap resolves itself: applied once against a local backend, then its state
migrated into the bucket it created.

**Step 4 — artefacts.** They sit outside the layers. Terraform owns the bucket
and the function's shape, not their contents. The pitfall worth the ink is that
a Lambda declared in Terraform will revert to its placeholder package on the
next apply unless `filename` and `source_code_hash` are under `ignore_changes` —
infrastructure and deploys otherwise overwrite each other silently. Recorded as
DR-0005 along with the layering.

**Steps 5–6.** DR-0004 and DR-0005 written as separate records; they are two
decisions, not one — the tool choice would survive a different layering and vice
versa. `docs/design/deployment.md` drafted and `docs/design/index.md` updated.

Two things surfaced while drafting `deployment.md` that were not anticipated by
the plan:

- **DR-0001's CORS gap is closed at API Gateway, not in `crates/server`.**
  HTTP APIs answer preflight themselves, and — the part that matters — they
  answer `OPTIONS` before the JWT authorizer runs, so preflight is not rejected
  for carrying no token. Configuring CORS in the service instead would have hit
  that wall. `crates/server` therefore stays without a CORS layer, which makes
  DR-0001's note about the gap resolved rather than outstanding.
- **`crates/server` binds `127.0.0.1:3000` as a constant.** Under the Lambda Web
  Adapter this is accommodated with `AWS_LWA_PORT=3000` rather than changed, but
  nothing checks the two agree. Recorded as a constraint in `deployment.md`.

`docs/design/index.md`'s "not yet written" list lost the deployment entry and
gained a **ci** entry: the pipeline was out of scope for this work, and the gap
is now visible rather than implied by deployment's absence.

## Verification

The output is documentation, so verification is a read-through against the rules
in `docs/README.md`, plus the user's confirmation.

Checked:

- All four documents are in English, including titles and slugs.
- DR numbering continues from DR-0003 with no reuse, and filenames match
  `DR-000N-<slug>.md`.
- `docs/design/index.md` lists deployment once, in the Documents table, and both
  new records in the Decision Records table. No dangling reference to a document
  that does not exist.
- Both constraints DR-0001 flagged for deployment to close — CORS and the
  403/404 SPA fallback — appear in `deployment.md`, each citing DR-0001 rather
  than restating its reasoning.
- Every alternative rejected during the evaluation lives in DR-0004 or DR-0005,
  not only here, so deleting this log loses nothing.
- No durable document cites this log.

Outstanding: the user confirming that the layer decomposition matches their
intent and that the records state the reasoning they actually hold. Design
Documents are overwritten by nature and `docs/README.md` puts them under human
confirmation, so the checklist below stays unticked until then.

## Retirement

- [ ] Design Documents updated — `deployment.md` written, `index.md` updated;
      awaiting confirmation
- [ ] Decision Records written (DR-0004, DR-0005)
- [ ] Non-obvious knowledge preserved — rejected alternatives, pitfalls, constraints
- [ ] No durable document depends on this log
