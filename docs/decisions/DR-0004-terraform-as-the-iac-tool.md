# DR-0004: Terraform is the Infrastructure as Code tool

Status: accepted
Date: 2026-08-02

## Context

The AWS infrastructure this project needs was fixed before any of it was
written: S3 and CloudFront delivering the CSR bundle (DR-0001), a Cognito User
Pool with Google as an identity provider, and an axum API running on Lambda
behind an API Gateway HTTP API with a JWT authorizer. Something has to create
all of it, and the choice determines what the layering in DR-0005 can even
express.

Three properties of this project narrow the field more than any general
comparison would:

- **The whole stack has to be reachable by one tool.** The infrastructure spans
  edge delivery, a managed identity provider, and a serverless API. A tool that
  covers one of those well and the others badly means running two tools.
- **The configuration must split into independently applied units with separate
  state.** This is the requirement DR-0005 exists to satisfy, and not every tool
  makes the boundary between units soft enough to move later.
- **Node.js is not in this project's toolchain.** `docs/design/frontend.md`
  records that the frontend build admits no Node.js and no npm anywhere — the
  CSS is hand-written for exactly that reason. Introducing a Node runtime for
  infrastructure would reverse a constraint the frontend paid for.

There is one environment and one cloud. Multi-cloud portability carries no
weight here, and neither does anything that only pays off across several
deployment environments.

## Decision

Use **HashiCorp Terraform**, with the AWS provider, under its BUSL 1.1 licence.

State lives in S3, one state file per layer, as described in DR-0005. Nothing
in the configuration is to use a Terraform-only feature where a portable
equivalent exists; see the note on OpenTofu below for why that restraint is
deliberate rather than incidental.

## Alternatives

**OpenTofu.** The only close call, and the alternative most likely to be
revisited. It is an MPL 2.0 fork that is drop-in compatible with the same AWS
provider, which removes the BUSL question entirely, and it has since added
things Terraform lacks — notably native state encryption. It was not chosen for
ecosystem gravity: provider documentation, published modules, worked examples,
and the accumulated body of answers to AWS-specific problems still centre on
Terraform, and this project will lean on that far more than it will on the
licence difference. Because the reasoning is about ecosystem rather than
capability, OpenTofu is recorded as a live escape hatch, and the configuration
avoids Terraform-only features so that taking it stays a low-cost change.

**AWS CDK, CDK for Terraform, and SST.** All rejected on the same ground: the
CDK CLI is a Node.js program, and CDKTF and SST are Node-centric in the same
way. This holds regardless of the authoring language — writing CDK in Python
still requires Node to synthesise. Adopting any of them reintroduces the
runtime that `docs/design/frontend.md` deliberately keeps out, and none offers
anything in return that this stack needs. CDK's other cost is inherited from
CloudFormation, below.

**Pulumi.** Rejected. There is no official Rust SDK, so it would not have given
the project a single language even if adopted, and the languages it does
support pull in either Node or a service-backed state model by default.
Self-managed backends exist, but the result is an additional moving part with
no benefit at this size.

**AWS SAM.** Rejected. It is genuinely good at the half of this stack that is
serverless — the Lambda, the HTTP API, the local invoke story — and has little
to say about the CloudFront distribution, the S3 origin with Origin Access
Control, or the Cognito User Pool and its Google federation. Using it would
mean SAM for the API and something else for everything else, which is the
outcome the first criterion above exists to avoid.

**Raw CloudFormation.** Rejected. Splitting a deployment across stacks relies
on exports, and an export that another stack imports cannot be changed while
the import exists. The layer boundaries this project wants are precisely the
things most likely to need adjusting as the system grows, so a mechanism that
freezes them is the wrong shape. Change sets are also a weaker review artefact
than `terraform plan`, and there is no module system worth the name.

## Consequences

Easy: every resource in the stack is reachable from one tool and one language;
`terraform plan` gives a reviewable diff before anything changes; the layering
in DR-0005 is expressible directly as separate root modules; and the volume of
existing AWS-specific Terraform material means most problems this project hits
have been solved in public already.

Hard, and accepted deliberately:

- **The BUSL licence.** Terraform is not open source. The licence restricts
  offering a competing Infrastructure as Code service, which this project never
  does, so the practical risk is close to zero — but it is a real difference
  from the rest of this project's dependencies and is recorded rather than
  glossed over.
- **State is this project's problem.** CloudFormation and CDK get state,
  locking, and drift handling from the platform for free. Terraform requires
  the state backend to be created, protected, and locked deliberately, and it
  requires a bootstrap step that has no home in any layer. DR-0005 resolves
  that.
- **Nothing checks the configuration against the application.** Terraform knows
  the Lambda exists; it does not know that `crates/server` binds the port the
  Lambda Web Adapter expects. That agreement is maintained by
  `docs/design/deployment.md` and by nothing else.

Reversing to OpenTofu would be cheap — the same configuration, a different
binary — and the restraint on Terraform-only features is what keeps it that
way. Reversing to a CloudFormation-based tool would mean rewriting every
resource and reintroducing Node.js, and is not contemplated.
