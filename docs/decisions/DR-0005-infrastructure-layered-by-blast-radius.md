# DR-0005: The infrastructure is layered by blast radius, not by environment

Status: accepted
Date: 2026-08-02

## Context

With Terraform chosen (DR-0004), the remaining question is how the
configuration is divided. The usual answer — a `prod` set and a `dev` set — does
not apply here, because there is only one environment. Development happens
locally, against `trunk serve` and `cargo run -p server`; nothing but production
exists on AWS.

That removes the conventional division and leaves the one that actually
matters. The resources in this stack differ enormously in what destroying them
costs:

- Destroying the Cognito User Pool destroys every user identity in it. Nothing
  recreates them.
- Destroying the CloudFront distribution costs an outage and yields a new
  `*.cloudfront.net` domain, which every registered callback URL and every CORS
  allow-list then has to be corrected to match.
- Destroying the Lambda and the HTTP API costs one redeploy from source.

They also differ in churn, and the two orderings are inverted: the resources
that are most dangerous to destroy are the ones that change least often, and the
resource that changes on every deploy is the one that is safe to lose. A single
state file puts all of them behind the same `terraform apply`, and therefore
behind the same mistake.

## Decision

Divide the configuration into **four root modules, each with its own state file
and its own apply**, ordered by what destroying them costs:

| Layer | Owns | Cost of destroying it |
| --- | --- | --- |
| `bootstrap` | The S3 bucket holding every other layer's state | Loses the record of everything else |
| `identity` | Cognito User Pool, Google identity provider, hosted-UI domain, the PKCE app client | Irreversible: user identities are gone |
| `delivery` | S3 origin bucket, CloudFront distribution, Origin Access Control, cache behaviour, the SPA fallback | Outage, plus a new domain every downstream reference must follow |
| `api` | Lambda function, API Gateway HTTP API, JWT authorizer, CORS, execution role, log groups | One redeploy from source |

**Dependencies run one way and form no cycle.** `delivery` produces the
CloudFront domain; `identity` needs it for its callback URLs; `api` needs the
issuer and client id from `identity` and the CloudFront domain for its CORS
allow-list. `delivery` itself depends on nothing above `bootstrap`, because the
SPA is static — the API URL and the Cognito client id reach it at build time,
not through Terraform. So the create order is `bootstrap`, `delivery`,
`identity`, `api`, and the destroy order is its reverse.

**Layers communicate through SSM Parameter Store, not through each other's
state.** Each layer publishes its outputs as parameters under a path of its own
and reads its inputs with the `aws_ssm_parameter` data source. No layer is
granted read access to another layer's state file.

**`bootstrap` resolves its own chicken-and-egg.** It is applied once with a
local backend, and its state is then migrated into the bucket it just created.
The bucket carries versioning, encryption, a public-access block, and
`prevent_destroy`.

**Application artefacts are outside the layers entirely.** Terraform owns the
S3 bucket and the Lambda function's shape; it does not own the bytes in either.
The WASM bundle and the Lambda package deploy on their own cadence, by
`aws s3 sync` and `aws lambda update-function-code`, and the Lambda's package
attributes are held under `ignore_changes` so that applies and deploys do not
overwrite each other.

The concrete parameter names, directory layout, and operational commands are in
`docs/design/deployment.md`.

## Alternatives

**Terraform workspaces, or per-environment variable files.** Not merely
deprioritised — ruled out, because the axis they divide along does not exist
here. Both answer "the same infrastructure, several times over," and this
project deploys it once. Reaching for them would have produced a `prod`
workspace and nothing else, with the blast-radius problem entirely unaddressed.

**A single root module for everything.** Rejected. It is the simplest thing that
works and it fails on exactly the case this decision exists to prevent: one
mistaken `terraform destroy`, or one badly-scoped `-target`, reaches the user
pool. The convenience of a single atomic apply is not worth an irreversible
failure mode.

**Splitting by AWS service, or by resource type.** Rejected. It produces
boundaries that have nothing to do with risk — the Lambda and the log group
separated, the CloudFront distribution and its origin bucket separated — and
multiplies the applies without containing anything.

**`terraform_remote_state` for cross-layer values.** The conventional idiom, and
rejected on two grounds. It requires the upper layer to be able to read the
lower layer's entire state file, which is far more access than it needs for two
strings, and it couples layers to state layout rather than to a declared
interface — refactoring a lower layer's internals can break an upper layer that
never asked about them. SSM parameters make the contract explicit and small, and
they have a second consumer: the SPA build and the deploy commands need the same
values, so publishing them once serves both.

**Hand-copied values in `.tfvars`.** Rejected. It decouples completely and rots
silently — nothing detects that the CloudFront domain in the identity layer's
variables stopped matching the real one until an authentication redirect fails.

**Putting `identity` and `delivery` in one layer.** Considered, since both are
low-churn. Rejected because it merges "irreversible" with "disruptive but
recoverable" into one blast radius, which is the distinction the whole decision
turns on.

## Consequences

Easy: a mistake is contained within one layer; the `api` layer, which changes
most often, can be applied freely without any path to the user pool; each layer
is small enough that its plan output can actually be read; and the parameters
published for cross-layer use are the same ones the application build needs.

Hard, and accepted deliberately:

- **A first create is four sequenced applies**, in dependency order, not one
  command. There is no tool-level enforcement of that order; it is documented in
  `docs/design/deployment.md` and it is on the operator to follow.
- **A change spanning layers is not atomic.** Adding a second frontend origin
  means applying `delivery`, then `identity`, then `api`, and the system is
  briefly inconsistent between them.
- **The SSM parameter names are an interface.** Renaming one silently breaks the
  layer that reads it, at plan time rather than at apply time, and only if that
  layer is planned. They are as much a contract as a function signature.
- **`bootstrap` cannot protect itself from everything.** `prevent_destroy` stops
  Terraform; it does not stop the console. Bucket versioning is what makes the
  remaining case recoverable.

Reversing the layering later is a state operation, not a rewrite: `terraform
state mv` between root modules merges or splits layers without touching the
resources. That is cheap enough that the layer set can be revisited when the
stack grows a database — which is the resource this scheme most obviously
anticipates and does not yet have.
