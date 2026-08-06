# Terraform setup — implementation plan

## Context

`docs/work/2026-08-02-terraform-setup.md` opened this unit of work and got as far
as step 1: `.devcontainer/Dockerfile` now pins Terraform 1.15.8 and copies in the
AWS CLI, and the container has been rebuilt. Both tools are confirmed working in
this session (`terraform 1.15.8`, `aws-cli 2.36.14`, credentials resolve to the
project's AWS account in `ap-northeast-1`).

What is missing is the configuration itself. DR-0004 chose Terraform, DR-0005
designed the four-layer split by blast radius, and `docs/design/deployment.md`
names every resource, SSM parameter path, and constraint — but no `.tf` file
exists. This plan turns that design into working configuration.

**Scope confirmed with the user (2026-08-02):** all four layers are authored,
**nothing is applied**. Verification stops at `terraform fmt -check` and
`terraform validate`. No AWS resource is created in this unit of work.

## Decisions settled before writing (work log step 2)

These are the implementation-level questions the durable layer deliberately left
open. Each is recorded here and will be appended to the work log's Progress.

| Question | Decision |
| --- | --- |
| **State locking** | S3 native `use_lockfile = true`. Terraform 1.15.8 supports it, DynamoDB locking is deprecated in the S3 backend, and OpenTofu supports it too — so it does not spend the escape hatch DR-0004 kept open. No DynamoDB table. |
| **State bucket name** | `rust-leptos-axum-aws-tfstate-<account_id>`, computed in `bootstrap` from `data.aws_caller_identity`. The account id must not enter the repository (user's call), so the literal lives in a **gitignored `infra/backend.hcl`**; `infra/backend.hcl.example` is committed. |
| **Google client id / secret** | Terraform `variable`s with no default, `sensitive = true` on the secret. Supplied via a gitignored `*.auto.tfvars` or `TF_VAR_*`. Never committed. |
| **Placeholder Lambda package** | `data "archive_file"` (hashicorp/archive) zips a committed `infra/api/placeholder/bootstrap` stub. Combined with `ignore_changes`, it is used at create only. |
| **Lambda Web Adapter layer** | `arn:aws:lambda:ap-northeast-1:753240598075:layer:LambdaAdapterLayerX86:28` (LWA 1.0.1). **Verified live** against the Lambda API in `ap-northeast-1` this session — the layer name is `LambdaAdapterLayerX86`, not `…X86_64`. Architecture is `x86_64`, matching the devcontainer's native Rust target. |
| **Version constraints** | `required_version = ">= 1.11.0"`, `hashicorp/aws ~> 6.0` (latest 6.57.1), `hashicorp/archive ~> 2.0`. |
| **Resource names** | Prefix `rust-leptos-axum-aws` throughout; see the table at the end. |

## File layout

```text
infra/
  backend.hcl.example      committed: bucket/region/encrypt/use_lockfile template
  backend.hcl              GITIGNORED: the real one, written after bootstrap applies
  bootstrap/               versions.tf variables.tf main.tf outputs.tf
    backend.tf.example     committed: the migration snippet (see below)
  delivery/                versions.tf backend.tf variables.tf main.tf outputs.tf
  identity/                versions.tf backend.tf variables.tf main.tf outputs.tf
  api/                     versions.tf backend.tf variables.tf iam.tf lambda.tf
                           apigateway.tf outputs.tf
    placeholder/bootstrap  committed stub, zipped by archive_file
```

Each layer's `backend "s3"` block declares only `key = "<layer>/terraform.tfstate"`;
`bucket`, `region`, `encrypt`, `use_lockfile` come from `-backend-config=../backend.hcl`.

`bootstrap` ships **no** backend block, so its first `init` is local. The
migration path is `cp backend.tf.example backend.tf` then
`terraform init -migrate-state -backend-config=../backend.hcl`.
`/infra/bootstrap/backend.tf` is gitignored so the committed tree always shows
the local-backend starting state.

## Step 3 — `infra/bootstrap`

- `aws_s3_bucket` named `${var.project}-tfstate-${data.aws_caller_identity.current.account_id}`,
  with `lifecycle { prevent_destroy = true }`.
- `aws_s3_bucket_versioning` (Enabled), `aws_s3_bucket_server_side_encryption_configuration`
  (AES256), `aws_s3_bucket_public_access_block` (all four flags true),
  `aws_s3_bucket_ownership_controls` (BucketOwnerEnforced).
- `aws_ssm_parameter` `/rust-leptos-axum-aws/bootstrap/state_bucket`, so the
  bucket name is discoverable without reading state.
- `output "state_bucket"` and `output "backend_hcl"` — the latter renders the
  exact `infra/backend.hcl` contents to paste after the first apply.

## Step 4 — `infra/delivery`

- Private origin bucket `${project}-spa-${account_id}`, with the same versioning /
  encryption / public-access-block / ownership set.
- `aws_cloudfront_origin_access_control` (type `s3`, sigv4, always sign).
- `aws_s3_bucket_policy` granting `s3:GetObject` to
  `cloudfront.amazonaws.com` conditioned on `AWS:SourceArn` = the distribution ARN.
- `aws_cloudfront_distribution`:
  - `default_root_object = "index.html"`, `price_class = "PriceClass_200"`,
    `cloudfront_default_certificate = true`, `geo_restriction` none.
  - **default behaviour** → managed `CachingOptimized` (hashed assets),
    `redirect-to-https`, `GET`/`HEAD` only.
  - **`ordered_cache_behavior` for `/index.html`** → managed `CachingDisabled`
    (`deployment.md`: caching the entry point serves a stale bundle reference).
  - **`custom_error_response` for both 403 and 404** → `/index.html`, status
    200, `error_caching_min_ttl = 0`. 403 matters because OAC makes a missing
    key return `AccessDenied`.
  - Managed policies resolved by `data "aws_cloudfront_cache_policy"`, not
    hard-coded ids.
- Publishes: `/…/delivery/cloudfront_domain`, `/…/delivery/cloudfront_distribution_id`,
  `/…/delivery/spa_bucket`.

## Step 5 — `infra/identity`

- Reads `/…/delivery/cloudfront_domain` via `data "aws_ssm_parameter"`.
- `aws_cognito_user_pool` — email as username attribute and auto-verified.
- `aws_cognito_identity_provider` `Google`, `authorize_scopes = "openid email profile"`,
  `attribute_mapping` for email/username, client id and secret from variables.
- `aws_cognito_user_pool_domain` prefix `rust-leptos-axum-aws-auth` (must be
  globally unique — flagged as an apply-time risk).
- `aws_cognito_user_pool_client`: `generate_secret = false` (public client,
  PKCE replaces the secret), `allowed_oauth_flows = ["code"]`,
  `allowed_oauth_scopes = ["openid","email","profile"]`,
  `supported_identity_providers = ["Google"]`, `depends_on` the IdP.
  `callback_urls` / `logout_urls` list **both** `https://<cloudfront_domain>/`
  and `http://localhost:8080/` — one client serves production and `trunk serve`.
- Publishes: `user_pool_id`, `user_pool_issuer`
  (`https://${aws_cognito_user_pool.this.endpoint}`), `app_client_id`,
  `hosted_ui_domain`.

## Step 6 — `infra/api`

- Reads `cloudfront_domain`, `user_pool_issuer`, `app_client_id` from SSM.
- `iam.tf`: execution role with the Lambda trust policy +
  `AWSLambdaBasicExecutionRole`.
- `lambda.tf`: `aws_cloudwatch_log_group` `/aws/lambda/rust-leptos-axum-aws-api`
  (14-day retention) created before the function; `aws_lambda_function` with
  `runtime = "provided.al2023"`, `handler = "bootstrap"`,
  `architectures = ["x86_64"]`, the LWA layer, and
  `environment` = `AWS_LAMBDA_EXEC_WRAPPER=/opt/bootstrap`, `AWS_LWA_PORT=3000`,
  `AWS_LWA_READINESS_CHECK_PATH=/health`.
  `lifecycle { ignore_changes = [filename, source_code_hash] }` — without it
  every apply reverts the function to the placeholder.
- `apigateway.tf`: `aws_apigatewayv2_api` (HTTP) with `cors_configuration`
  (`allow_origins = ["https://<cloudfront_domain>"]`, methods `GET`/`POST`/`OPTIONS`,
  headers `authorization`/`content-type`); `AWS_PROXY` integration with payload
  format `2.0`; `aws_apigatewayv2_authorizer` (JWT, issuer + audience from
  identity, `identity_sources = ["$request.header.Authorization"]`);
  `$default` stage with `auto_deploy` and access logging to its own log group;
  `aws_lambda_permission` for the API to invoke.
- **Routes:** `ANY /api/{proxy+}` behind the JWT authorizer, and `GET /health`
  left public so the platform can probe it without a token.
- Publishes: `/…/api/api_endpoint`, `/…/api/lambda_function_name`.

**Flagged, not resolved here:** the SPA has no authentication code yet
(`crates/app/src/api.rs` sends no `Authorization` header), so once this is
applied `GET /api/greeting` will return 401. That follows from `deployment.md`,
which specifies the authorizer; closing it is frontend work, not Terraform work.
It will be recorded in the work log rather than silently designed around.

## Steps 7–8 — repo plumbing

`.gitignore` gains:

```text
.terraform/
*.tfstate
*.tfstate.*
*.tfvars
!*.tfvars.example
/infra/backend.hcl
/infra/bootstrap/backend.tf
```

`.terraform.lock.hcl` is **committed** in each layer — it is the provider pin.

`justfile` gains, alongside the existing `check`/`lint`/`fmt` recipes:

```text
tf-init LAYER      terraform -chdir=infra/{{LAYER}} init -backend-config=../backend.hcl
tf-fmt             terraform fmt -recursive infra
tf-fmt-check       terraform fmt -recursive -check infra
tf-validate        init -backend=false + validate, each layer in turn
tf-plan LAYER      terraform -chdir=infra/{{LAYER}} plan
tf-apply LAYER     terraform -chdir=infra/{{LAYER}} apply
```

with the apply order — `bootstrap`, `delivery`, `identity`, `api` — written in a
comment above them, since nothing enforces it (DR-0005).

## Steps 10–11 — durable layer

- **DR-0006** — *The Terraform state backend is configured from outside the
  repository.* Covers native `use_lockfile` over a DynamoDB table, the account
  id kept out of the tree via a gitignored `backend.hcl`, and `bootstrap`
  shipping a local backend with a documented migration. Real alternatives were
  weighed for each, and reversing them is a state operation.
- **`docs/design/deployment.md`** — replace the illustrative `infra/` tree with
  the real one, and add the constraints this work introduces: the pinned LWA
  layer ARN and its `LambdaAdapterLayerX86` name, the `x86_64` architecture, the
  placeholder package, `backend.hcl` being uncommitted, and the public
  `/health` route.
- **`docs/design/index.md`** — add DR-0006 to the record table.
- Both design-document edits are **drafted and confirmed with the user** before
  the work is marked complete (`docs/README.md`: design documents are
  overwrite-oriented and need human confirmation).

The work log's Progress, Verification, and Retirement sections are filled in as
this proceeds; the log itself is not deleted in this unit of work.

## Verification

Network to `registry.terraform.io` was confirmed reachable from this container
(HTTP 200), so provider schemas can be fetched without credentials:

1. `just tf-fmt-check` — all four layers pass `terraform fmt -check`.
2. Per layer: `terraform -chdir=infra/<layer> init -backend=false` then
   `terraform validate`. No AWS credentials are used and no backend is
   contacted.
3. Read-through against `docs/design/deployment.md`, confirming every resource,
   every one of the nine SSM parameter paths, and every Constraint bullet has a
   corresponding line in the configuration.

`validate` is the ceiling. It proves the configuration is internally consistent
and matches the provider schema. It does not prove the infrastructure works,
that IAM permits what the layers need, or that the apply order holds — and two
names are only checkable at apply time: the S3 bucket names and the Cognito
domain prefix must be globally unique.

## Resource names

| Thing | Name |
| --- | --- |
| Project prefix / SSM root | `rust-leptos-axum-aws` |
| State bucket | `rust-leptos-axum-aws-tfstate-<account_id>` |
| SPA origin bucket | `rust-leptos-axum-aws-spa-<account_id>` |
| Origin Access Control | `rust-leptos-axum-aws-spa` |
| Cognito user pool | `rust-leptos-axum-aws` |
| Hosted UI domain prefix | `rust-leptos-axum-aws-auth` |
| Cognito app client | `rust-leptos-axum-aws-spa` |
| Lambda function | `rust-leptos-axum-aws-api` |
| Lambda execution role | `rust-leptos-axum-aws-api-lambda` |
| HTTP API | `rust-leptos-axum-aws-api` |
| Log groups | `/aws/lambda/rust-leptos-axum-aws-api`, `/aws/apigateway/rust-leptos-axum-aws-api` |

All names come from `var.project`, so changing the prefix later is a one-line
edit plus a recreate.
