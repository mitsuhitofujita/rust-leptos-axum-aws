# Finish the DynamoDB persistence work log

## Context

`docs/work/2026-08-09-dynamodb-persistence-infrastructure-and-schema.md` is an
open Work Log. Its steps 1–7 are already written, verified and committed across
two commits (`9703140` docs, `58d5ddc` infra) — `infra/data` exists as the fifth
root module, `infra/api` reads the table's two SSM parameters, the `justfile`
and `docs/design/deployment.md` carry the new apply order, and DR-0015, DR-0016
and `docs/design/persistence.md` are in place.

**Step 8 alone is unfinished**: `data` has never been applied, so its two SSM
parameters do not exist, and `infra/api` — which now reads them with
`aws_ssm_parameter` data sources — cannot even be planned until they do. The log
recorded the reason: the devcontainer's AWS session had expired. It still has
(`aws sts get-caller-identity` → "Your session has expired").

The outcome wanted here: apply the two layers for real, replace the "never
applied" notes in the durable documents with what is actually true, and retire
the Work Log per `docs/README.md`.

The user additionally confirmed fixing the staleness the log noticed but left
alone: `docs/design/index.md` still describes the backend as serving
`GET /api/greeting` returning `shared::Greeting`, and says "All four layers have
been applied".

## Prerequisite — the user must reauthenticate

The AWS CLI here uses `aws login` (IAM Identity Center, profile in
`~/.aws/config` pointing at the `AdministratorAccess` SSO role, region
`ap-northeast-1`). It is interactive, so it cannot be run from a tool call.

Before step 2 the user runs, in this session:

```
! aws login
```

Everything below waits on that. No tooling needs to be written for this work —
no Python, no new Rust helper; `terraform`, `just` and `aws` cover it.

## Steps

### 1. Pre-flight checks (no credentials needed)

- `just tf-fmt-check` over the whole `infra` tree.
- `terraform -chdir=infra/data validate` and the same for `infra/api`.
  `just tf-validate` still cannot run end to end for the reason the log already
  recorded — `bootstrap` and `api` carry `.terraform` directories from a real
  backend `init`, and `init -backend=false` there still resolves the stored S3
  backend. Once credentials are restored in step 2 this stops mattering, so run
  `just tf-validate` again after login and record whichever result it gives.

### 2. Apply `data`

```
just tf-init data      # infra/data/.terraform exists but was never backend-initialised
just tf-plan data      # expect exactly 3 creates: the table and its two parameters
just tf-apply data
```

Expected plan: `aws_dynamodb_table.app`, `aws_ssm_parameter.table_name`,
`aws_ssm_parameter.table_arn` — three to add, none to change or destroy. Anything
else in the plan is a signal to stop and read it rather than approve.

Note that `main.tf` carries `lifecycle { prevent_destroy = true }` and
`deletion_protection_enabled = true`: after this apply the table cannot be
removed by `terraform destroy` without editing both. That is intended
(DR-0005 blast-radius split), but it is the point of no easy return in this
plan.

### 3. Re-apply `api`

```
just tf-plan api
just tf-apply api
```

Expected changes, all from commit `58d5ddc`:

- `aws_iam_role_policy.lambda_table` created — the inline policy in
  `infra/api/iam.tf`, scoped to `local.table_arn`, with `Scan` deliberately
  absent.
- `aws_lambda_function.api` updated in place — `TABLE_NAME` added to
  `environment.variables` in `infra/api/lambda.tf`.

Nothing about the HTTP API, the authorizer or the function's code should move.

### 4. Verify against the real account

- `aws dynamodb describe-table --table-name rust-leptos-axum-aws-app` — confirm
  `BillingMode` `PAY_PER_REQUEST`, key schema `pk`/`sk` both `S`, no
  `GlobalSecondaryIndexes`, `DeletionProtectionEnabled` true.
- `aws dynamodb describe-continuous-backups --table-name rust-leptos-axum-aws-app`
  — confirm point-in-time recovery is `ENABLED`.
- `just _ssm data/table_name` and `just _ssm data/table_arn` — both resolve.
- `aws lambda get-function-configuration --function-name "$(just _ssm api/lambda_function_name)"`
  — `TABLE_NAME` present in the environment.
- `aws iam get-role-policy --role-name rust-leptos-axum-aws-api-lambda --policy-name rust-leptos-axum-aws-api-table`
  — the resource is the table ARN alone and the action list has no `Scan`.
- `curl "$(just _ssm api/api_endpoint)health"` still returns `ok`, confirming the
  function still starts after the environment change.

No write is issued against the table: nothing reads or writes it yet, and this
unit of work deliberately stops at the substrate.

### 5. Update the durable documents that claim the layer is unapplied

- **`docs/design/deployment.md`** — the note at lines 5–13. Fold `data` into the
  applied set, correct the parameter count (ten becomes twelve), and delete the
  paragraph saying `data` "has never been applied" and that `api` "cannot be
  planned until `data` has been applied once". Keep the CloudFront-bundle
  sentence, which is unrelated and still true. Bump `Updated:`.
- **`docs/design/persistence.md`** — the note at lines 5–10 opens "the table
  exists as configuration and nothing reads or writes it yet". The first clause
  is now false and the rest is still true: rewrite it to say the table exists,
  and that `crates/server` still answers `GET /api/dashboard` from hardcoded
  values, holds no AWS SDK dependency, and does not yet extract the Cognito
  `sub`. Bump `Updated:`.
- **`docs/design/index.md`** — the user-confirmed fix, kept to facts:
  `GET /api/greeting` returning a `shared::Greeting` becomes `GET /api/dashboard`
  returning a `shared::Dashboard` (`crates/server/src/main.rs:23,36`;
  `crates/shared/src/lib.rs:54`), and "All four layers have been applied" in the
  **ci** entry becomes five. Bump `Updated:`.

These are Design Document overwrites, which `docs/README.md` says a human
confirms. Present the three diffs before committing them.

### 6. Close and retire the Work Log

Append a dated `2026-08-09` Progress entry recording the apply, the observed
plan, and anything the plan above did not predict. Update the Verification
section with the real `just tf-validate` result and the account-level checks.
Set `Status: complete` and tick all four Retirement boxes, naming DR-0015 and
DR-0016.

Then delete `docs/work/2026-08-09-dynamodb-persistence-infrastructure-and-schema.md`.
The checklist holds: the Design Documents carry the resulting state, the two
Decision Records carry the reasoning, and `rg` over `docs/design`,
`docs/decisions` and `README.md` finds no citation of the log.

If step 2 or 3 turns up something the durable layer cannot express — a rejected
alternative or a non-obvious constraint — it goes into a Decision Record before
the log is deleted, not into the commit message.

### 7. Commit

Two commits, Conventional Commits, using the `/commit` skill:

1. `docs: retire the DynamoDB persistence log` — the three Design Document
   updates and the deleted Work Log.
2. A separate commit for the `index.md` backend/ci correction if it reads
   cleanly on its own; fold it into the first if splitting is artificial.

No Terraform file changes in either — the apply consumes the configuration
already committed, and state lives in S3, not the repository.

## Verification

The work is done when all of the following hold:

- `just tf-plan data` and `just tf-plan api` both report **no changes** — the
  configuration and the account agree.
- `just _ssm data/table_name` prints `rust-leptos-axum-aws-app`, and
  `just _ssm data/table_arn` prints its ARN.
- `describe-table` shows the on-demand, two-key, index-free table with deletion
  protection on, and `describe-continuous-backups` shows PITR `ENABLED`.
- The Lambda's configuration lists `TABLE_NAME`, its role carries the inline
  table policy, and `GET /health` on the API endpoint still returns `ok`.
- `just tf-fmt-check` passes and `git status` is clean after the commits.
- `docs/work/` is empty, and no durable document claims `data` is unapplied
  (`rg -n "never been applied|four layers" docs/`).

## Risks

- **The apply is not reversible by `terraform destroy`.** Both destroy guards
  are on by design; removing the table later means editing `infra/data/main.tf`
  first. Flagged, not worked around.
- **`api`'s plan could show more than the two expected changes** if the deployed
  function has drifted from the configuration (its code is pushed by
  `just deploy-api`, outside Terraform). Read the plan before approving; the
  function's `filename`/`source_code_hash` is the field to check.
- **The SSO session can expire mid-apply.** If it does, re-run `aws login` and
  re-plan rather than assuming the partial state.
