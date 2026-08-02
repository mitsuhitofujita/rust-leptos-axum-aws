# Execute the IaC selection work log

## Context

`docs/work/2026-08-02-iac-selection.md` is an open Work Log whose Request,
Interpretation, and Plan are already settled and confirmed with the user. Steps 1
of its plan is done; steps 2–6 are not. This plan executes those steps.

The unit of work answers two questions: which IaC tool this project uses for its
AWS infrastructure, and how that configuration is decomposed. The decomposition
axis is **risk and blast radius**, not deployment environment — there is exactly
one environment, and development happens locally against `trunk serve` and
`cargo run -p server`. Each layer is an independent root module with its own
state file, so a mistaken destroy is contained.

**The deliverable is documentation, not code.** No `.tf` files, no CI/CD
pipeline, no AWS resources are produced. The work ends when DR-0004, DR-0005,
`docs/design/deployment.md`, and the index update exist and the user has
confirmed them.

Decisions the user settled during planning:

| Fork | Chosen |
| --- | --- |
| Tool | HashiCorp Terraform (BUSL 1.1) |
| Cross-layer outputs | SSM Parameter Store, read via the `aws_ssm_parameter` data source |
| Layer set | Four root modules: `bootstrap` → `delivery` → `identity` → `api` |

## The design being written down

### Layers

| Layer | Owns | Why it is its own state |
| --- | --- | --- |
| `bootstrap` | The S3 bucket holding every other layer's state; versioning, encryption, public-access block, `prevent_destroy` | Nothing can depend on it having been created by something that needs it |
| `delivery` | S3 origin bucket for `dist/`, CloudFront distribution, Origin Access Control, cache policies, the 403/404 → `/index.html` custom error responses | Destroy means downtime and a new `*.cloudfront.net` domain that every downstream reference has to follow |
| `identity` | Cognito User Pool, Google identity provider, hosted-UI domain, the PKCE public app client | Destroy is irreversible: user identities are gone. Highest-severity blast radius, lowest churn |
| `api` | Lambda function (Rust + Lambda Web Adapter), API Gateway HTTP API, JWT authorizer, CORS configuration, execution role, log groups | Fully reproducible from source. Highest churn, so it gets the state file that is safe to break |

Dependency direction is forced by the resources themselves and admits no cycle:

```text
bootstrap
   │
   ▼
delivery ── cloudfront_domain ──▶ identity ── issuer, client_id ──▶ api
   └──────── cloudfront_domain (CORS allowed origin) ─────────────────▶
```

`delivery` depends on nothing above `bootstrap` because the SPA is static: the
API URL and the Cognito client id reach it at build time, not through Terraform.

### Cross-layer contract

Each layer publishes its outputs as SSM parameters under `/<project>/<layer>/`
and reads its inputs with `data "aws_ssm_parameter"`. No layer reads another
layer's state file. The parameters to be named in `deployment.md`:

- `delivery`: `cloudfront_domain`, `cloudfront_distribution_id`, `spa_bucket`
- `identity`: `user_pool_id`, `user_pool_issuer`, `app_client_id`, `hosted_ui_domain`
- `api`: `api_endpoint`, `lambda_function_name`

The same parameters are the source of the SPA's build-time configuration and of
the deploy commands, so publication serves two consumers with one mechanism.

### Bootstrap

`bootstrap` is applied once with a local backend, then its own state is migrated
into the bucket it just created. The bucket carries `prevent_destroy`,
versioning, and SSE. The document records the alternative — creating the bucket
with a one-line `aws s3api` command and keeping it out of Terraform entirely —
and why it was not chosen: the bucket's versioning and encryption settings are
worth having declared rather than living in a README.

### Artefacts

`dist/` and the Lambda package deploy on a different cadence from the
infrastructure and are **not** Terraform-managed content. Terraform owns the
bucket and the function's *shape*; the artefacts are pushed by `aws s3 sync` +
CloudFront invalidation and `aws lambda update-function-code`. The Lambda
resource is created with a placeholder package and `lifecycle { ignore_changes =
[filename, source_code_hash] }`, or every `terraform apply` fights the last
deploy. This is the single most-missed detail in this shape and gets stated
plainly.

## Work

### 1. `docs/decisions/DR-0004-terraform-as-the-iac-tool.md` (new)

Follows the Decision Record template in `docs/README.md` (Context / Decision /
Alternatives / Consequences). Status: accepted. Date: 2026-08-02.

The Alternatives section is the part that must survive the Work Log's deletion —
each candidate rejected on its merits, not ignored:

- **OpenTofu** — the only close call. MPL 2.0 removes the BUSL question, and it
  is drop-in on the same AWS provider. Not chosen for ecosystem gravity;
  recorded as a deliberate escape hatch, which is why the design avoids
  Terraform-only features.
- **AWS CDK / CDKTF / SST** — the CDK CLI requires Node.js regardless of the
  authoring language. `docs/design/frontend.md` records that the build admits no
  Node.js or npm anywhere; these inherit that as a cost with no offsetting gain.
- **Pulumi** — no official Rust SDK, and the non-Node languages still pull in a
  service-backed state model by default. Extra moving part for no benefit at
  this size.
- **AWS SAM** — scoped to serverless. It handles the Lambda and the HTTP API
  well and the CloudFront/Cognito/S3 half badly, which would mean two tools.
- **Raw CloudFormation** — stack splitting relies on exports, and an export that
  another stack imports cannot be changed. That is precisely the layer boundary
  this project needs to keep soft.

Consequences: BUSL is accepted as a risk that does not bite this project (it
restricts offering a competing IaC service); state and locking become this
project's problem rather than the platform's; reversal to OpenTofu is cheap and
deliberately kept so.

### 2. `docs/decisions/DR-0005-infrastructure-layered-by-blast-radius.md` (new)

Same template. Records the four layers, the dependency direction, the SSM
contract, the bootstrap resolution, and the artefact separation.

Alternatives to record: Terraform workspaces and per-environment tfvars (ruled
out entirely — there is one environment, so the axis does not exist);
`terraform_remote_state` (rejected: grants read of the whole lower state and
couples to state layout rather than a declared interface); a single root module
(rejected: one `destroy` reaches the user pool); hand-copied tfvars (rejected:
rots silently).

Consequences to record: four applies must be sequenced by hand on a first
create; a change spanning layers is no longer atomic; SSM parameters are a
public-ish contract that has to be versioned deliberately.

### 3. `docs/design/deployment.md` (new)

Follows the Design Document template (Purpose / Structure / Interfaces /
Constraints), written in the present tense as the authoritative description.

**Structure** — the layer table and dependency diagram above, plus the directory
shape the future Terraform will take (`infra/bootstrap/`, `infra/delivery/`,
`infra/identity/`, `infra/api/`, each a root module with its own backend block).

**Interfaces** — the published SSM parameters; the runtime shape of the API
(Rust binary named `bootstrap` in a zip on `provided.al2023`, the Lambda Web
Adapter attached as a layer, `AWS_LAMBDA_EXEC_WRAPPER=/opt/bootstrap` and the
port variable pointing at the address `crates/server` binds); the deploy
commands for `dist/` and the Lambda package.

**Constraints**, each citing its record:

- CloudFront maps both **403 and 404** to `/index.html` with status 200 — DR-0001.
  403 matters specifically because the bucket is private behind OAC, so a missing
  key returns 403, not 404.
- The API must send CORS headers with the CloudFront domain as the allowed
  origin. `crates/server` has no CORS layer today; `docs/design/index.md` and
  DR-0001 both record this as an open gap that deployment closes.
- One Cognito app client serves both production and local development, so its
  callback URLs list the CloudFront domain **and** `http://localhost:8080`. This
  is a property of the `identity` layer, not a second environment.
- The app client is public and uses Authorization Code Flow with PKCE — no client
  secret, because the secret cannot be held in a WASM bundle.
- `index.html` is served with a short/no-cache header while hashed assets get a
  long one; otherwise a deploy serves a stale entry point pointing at hashed
  files that still exist.
- The Lambda's `filename`/`source_code_hash` are under `ignore_changes`.
- The state bucket carries `prevent_destroy`.

### 4. `docs/design/index.md` (edit)

Move **deployment** out of the "Not yet written" list into the Documents table;
add DR-0004 and DR-0005 to the Decision Records table; bump `Updated:`. Leave the
**backend** entry where it is — it is still unwritten, and its note about the
missing CORS layer stays accurate until the API actually gains one.

### 5. `docs/work/2026-08-02-iac-selection.md` (edit)

Append a dated Progress entry recording the evaluation and the design as they
were reached — the log is append-oriented, so the existing Interpretation and
Plan are not rewritten. Fill the Verification section. Fill the Retirement
checklist with the DR numbers, leaving the boxes unticked until the user
confirms.

### 6. Retire the log — only after confirmation

`docs/README.md` puts Design Documents under human confirmation before an
overwrite counts as complete, and the Work Log's own plan repeats it. So: present
the three new documents, and on the user's confirmation tick the checklist and
delete `docs/work/2026-08-02-iac-selection.md`, since git retains it. **Do not
delete it unprompted.**

## Verification

The output is documentation, so verification is a read-through against the rules
the project already wrote down:

1. Every document is in English, including titles and slugs — `docs/README.md`.
2. Filenames match `DR-000N-<slug>.md` and the numbers continue from DR-0003
   without reuse.
3. `docs/design/index.md` has no dangling reference: deployment appears once, in
   the table, and both new DRs are listed.
4. `deployment.md` reads as a description of the present, not a narrative of how
   the choice was made, and cites DR-0001/0004/0005 rather than retelling them.
5. Every constraint DR-0001 flagged for deployment to close — CORS, the SPA
   fallback — appears in `deployment.md`. Grep for `CORS` and `403` across
   `docs/design/` and confirm both land there.
6. The Work Log's retirement checklist is answerable: nothing in the three new
   documents cites the Work Log, and every rejected alternative from the
   evaluation lives in a DR rather than only in the log.
7. The user confirms the layer decomposition matches their intent — the
   Verification section of the Work Log names this as the actual test.
