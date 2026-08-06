# Set up the Terraform configuration for the AWS infrastructure

Status: complete
Started: 2026-08-02
Branch: main

## Request

Carry out the Terraform setup, taking `docs/work/2026-08-02-iac-selection.md` as
the basis for it.

That log selected the tool and designed the layering but deliberately produced no
code; this request is the execution of what it designed. The design it settled —
and therefore the input to this work — is recorded durably in DR-0004, DR-0005,
and `docs/design/deployment.md`.

### Clarifications

Asked on 2026-08-02, before any code was written.

- **All four layers are written, and nothing is applied.** `bootstrap`,
  `delivery`, `identity` and `api` are all authored in this unit of work. The
  work stops at `terraform fmt` and `terraform validate`; no `terraform apply`
  runs and no AWS resource is created.
- **The Terraform CLI is added to the devcontainer**, pinned by an `ARG` in
  `.devcontainer/Dockerfile`, in the same manner as trunk. The AWS CLI is added
  alongside it, since the artefact deployment described in `deployment.md`
  depends on it.
- **The region is `ap-northeast-1`.**
- **The project name is `rust-leptos-axum-aws`**, used as the resource name
  prefix and as the root of the SSM parameter paths.

Arriving later, each widening the scope set above.

- **2026-08-04 — apply the layers**, reversing the "nothing is applied" clause.
- **2026-08-04 — write the artefact deploy recipes, but do not run them.**
- **2026-08-05 — run both deploys**, reversing the clause above; then find out
  why the deployed page fails and fix it if it can be fixed. Documentation, code
  and infrastructure are all in scope for the investigation.
- **2026-08-05 — the greeting is not to be made to work here.** Its remaining
  gap is the SPA's missing access token, and that is a separate unit of work. The
  user also stated that asking for the deploys inside this log was a mis-scoping
  on their part, and asked that the records be reconciled with it rather than
  written as though the scope had always been this wide.

## Interpretation

**What is being asked.** Turn the deployment design into working Terraform: an
`infra/` tree of four independent root modules matching the layer set in DR-0005,
with the resources, the cross-layer SSM contract, and the constraints listed in
`docs/design/deployment.md` all expressed in configuration. Add the tooling that
makes that configuration checkable, and check it.

**What is out of scope.**

- Any `terraform apply`, `terraform init` against a real backend, or contact
  with an AWS account. The `bootstrap` state migration described in DR-0005 is
  configured for, not performed.
- The CI pipeline. `docs/design/index.md` already names **ci** as a document that
  does not exist; this work does not close that gap.
- The artefact deployment tooling — the `dist/` sync, the CloudFront
  invalidation, the Lambda code update, and the SSM-to-`trunk build` wiring.
  DR-0005 places artefacts outside the layers entirely, so they are outside
  a Terraform setup too. `deployment.md` already describes the commands.
- Changing `crates/server`. Its `127.0.0.1:3000` constant is accommodated by
  `AWS_LWA_PORT`, as `deployment.md` records; this work does not revisit that.
- Re-opening any choice in DR-0004 or DR-0005. They are inputs here, not
  questions.

**What is assumed.**

- The durable layer is the specification. Where `deployment.md` names a
  resource, a parameter path, or a constraint, the configuration implements it
  literally rather than improving on it. Anything that turns out to be wrong in
  `deployment.md` is reported and fixed there, not silently diverged from.
- `docs/work/2026-08-02-iac-selection.md` is still open with unticked retirement
  boxes. That is its own unfinished business — a pending user confirmation on
  the documents it wrote — and is not blocking here, because this work depends
  on the durable records rather than on that log.
- The design leaves several implementation-level questions open, and they have
  to be settled before any `.tf` file compiles. Each is listed in step 2 of the
  plan. They are genuinely undetermined by the durable layer rather than
  oversights in it, since DR-0005 deliberately stopped at the layer boundary.
- The devcontainer cannot be rebuilt from inside itself, so adding the pinned
  `ARG` to the Dockerfile does not put `terraform` on this session's `PATH`. To
  verify anything in this session, the same pinned version is also installed
  into the running container. The Dockerfile change is the durable half; the
  in-session install is scaffolding and is not committed.
- `terraform validate` needs the AWS provider schema, which means
  `terraform init -backend=false` and therefore network access to the registry.
  No AWS credentials are needed for it. If the registry is unreachable from this
  environment, verification degrades to `terraform fmt` plus review, and that is
  reported rather than papered over.
- Real values that cannot be committed — the Google OAuth client id and secret
  above all — are declared as inputs, never written into the repository.

## Plan

1. Add `terraform` and the AWS CLI to `.devcontainer/Dockerfile`, each pinned by
   an `ARG`. Install the same Terraform version into the running container so the
   rest of this plan can be verified without a rebuild.
2. Settle the implementation-level questions the durable layer leaves open, and
   record each with its reasoning in Progress:
   - **The state bucket's name.** A backend block cannot take variables, so the
     name is a literal in four places and has to be globally unique.
   - **State locking.** Whether to use the S3 native lock file or a DynamoDB
     table, judged against DR-0004's restraint on Terraform-only features.
   - **How the Google client id and secret reach `identity`** without entering
     the repository.
   - **The placeholder Lambda package**, which has to exist for the first apply
     even though DR-0005 puts the real artefact outside Terraform.
   - **The Lambda Web Adapter layer**, whose ARN is region-specific and needs a
     pinned version.
   - **The provider and Terraform version constraints.**
3. Write `infra/bootstrap`: the state bucket, with versioning, encryption, a
   public-access block, and `prevent_destroy`, plus a local backend and the
   documented migration path.
4. Write `infra/delivery`: the private origin bucket, Origin Access Control, the
   CloudFront distribution with the 403/404 → `/index.html` 200 mapping, the
   split cache behaviour for `index.html` against hashed assets, and the three
   SSM parameters it publishes.
5. Write `infra/identity`: the user pool, the Google identity provider, the
   hosted-UI domain, and the public PKCE app client with callback and logout URLs
   covering both the CloudFront domain and `http://localhost:8080`, plus the four
   SSM parameters it publishes.
6. Write `infra/api`: the execution role, the log groups, the Lambda with the
   adapter layer and its environment variables and `ignore_changes`, the HTTP
   API, the JWT authorizer, the CORS configuration, and the two SSM parameters it
   publishes.
7. Extend `.gitignore` for `.terraform/`, state files, and variable files.
8. Add `just` recipes for the per-layer `fmt`, `validate`, and `plan`, so the
   apply order that nothing enforces is at least written down where it is run.
9. Verify: `terraform fmt -check` and `terraform validate` in each layer.
10. Write a Decision Record for whichever of step 2's choices carry durable
    consequences, rather than leaving them only in this log.
11. Update `docs/design/deployment.md` so it describes what now exists — the real
    file layout and any constraint step 2 introduced — and update
    `docs/design/index.md` if the record table changes. Both need confirmation
    before this work is marked complete.

## Progress

### 2026-08-02

Work Log opened. Read `docs/README.md`, `docs/design/index.md`,
`deployment.md`, `workspace.md`, `frontend.md`, DR-0004 and DR-0005, and the IaC
selection log this request names.

Found before planning: neither `terraform` nor `aws` is installed in the
devcontainer, and `.devcontainer/Dockerfile` installs only trunk. Tooling is
therefore part of this work rather than a precondition of it.

Four questions raised with the user — scope, tooling, region, project name — and
their answers recorded under Clarifications. The scope answer is what shapes the
rest: all four layers, no apply, so the ceiling on verification is
`terraform validate` and the design's correctness against AWS remains unproven
until someone applies it.

Interpretation and Plan presented for confirmation. No configuration written yet.

Plan confirmed. Step 1 started.

**Step 1 — tooling.** Terraform 1.15.8 is the current release and is what the
Dockerfile now pins, in the same shape as the trunk block above it: an `ARG`, a
prebuilt binary, a fixed URL.

Two findings changed how the step was done:

- **HashiCorp ships the binary only as a zip, and this image has no unzip.** The
  trunk block gets away with `tar -xz` piped from curl; a zip cannot be streamed
  that way, so `unzip` joins the apt list. Installing it is cheaper than adding
  HashiCorp's apt repository, which would have brought a GPG key and a looser
  grip on the version.
- **Terraform goes to `/usr/local/bin`, not `${CARGO_HOME}/bin`.** trunk lives in
  the cargo path because it is part of the Rust build; Terraform is not, and it
  is installed before the `USER` switch alongside the AWS CLI, where system tools
  belong.

Superseded: the plan's step 1 also called for installing Terraform into the
running container so the rest of the work could be verified without a rebuild.
It was done — a perl one-liner substituted for the missing unzip — and then
dropped at the user's request in favour of rebuilding the container properly.
Nothing from it entered the repository.

The user added the AWS CLI to the Dockerfile themselves, as a `COPY --from=aws-cli`
from a stage that the Dockerfile does not declare and `devcontainer.json` does not
supply as a build context. Raised with them before the rebuild; they added
`FROM docker.io/amazon/aws-cli:latest AS aws-cli` in response, so the image now
builds. Step 1 is complete pending that rebuild.

**Handoff, written before the container is rebuilt.** The rebuild ends the
session that did the work above, so what the next one would otherwise have to
re-derive is recorded here.

State of the tree: `.devcontainer/Dockerfile` is modified and uncommitted; this
log is new and uncommitted; nothing under `infra/` exists yet.

What is already known about step 2, none of it yet decided:

- **Locking.** The pinned Terraform is 1.15.8, which supports the S3 backend's
  native `use_lockfile`. That removes the DynamoDB lock table the older idiom
  requires, and OpenTofu supports it too, so it does not spend the escape hatch
  DR-0004 kept open. This looks like the answer; it has not been confirmed
  against the provider documentation.
- **The state bucket's name.** A `backend` block takes no variables and no
  interpolation, so the name is a literal in three layers plus an output in
  `bootstrap`. S3 bucket names are globally unique, and the usual fix — an
  account id suffix — needs an account id nobody here has. A per-layer
  `-backend-config` file, gitignored, keeps the literal out of the repository
  and the name out of the configuration.
- **The Lambda Web Adapter layer ARN** is both region-specific and
  account-specific to AWS's own publishing account. It must be read from the
  adapter's current documentation for `ap-northeast-1` rather than recalled.
- **Verification reaches the network.** `releases.hashicorp.com` responded from
  this container, so `terraform init -backend=false` should be able to fetch the
  provider schema. No AWS credentials exist here, and none are needed for it.

Settled values, from the Clarifications above: region `ap-northeast-1`, project
name `rust-leptos-axum-aws`, layers at `infra/bootstrap`, `infra/delivery`,
`infra/identity`, `infra/api`.

Next action: step 2, then steps 3–6 in dependency order.

**Session resumed after the rebuild.** `terraform` 1.15.8 and `aws` 2.36.14 are
both on `PATH`, credentials resolve to the project's AWS account in
`ap-northeast-1`, and `registry.terraform.io` answers from the container. Step 1
is therefore complete and the verification ceiling described above is reachable.

Scope reconfirmed with the user before step 2: all four layers are written and
**nothing is applied**. The user also settled two of step 2's questions
directly — the AWS account id must not be committed to the repository, and the
Google credentials are passed as Terraform variables.

**Step 2 — the implementation-level questions, settled.**

- **State locking: the S3 native lock file.** `use_lockfile = true` in the
  backend block, no DynamoDB table. The pinned Terraform supports it, the S3
  backend's `dynamodb_table` argument is documented as deprecated and slated for
  removal, and OpenTofu implements the same argument — so it costs nothing
  against DR-0004's restraint on Terraform-only features.

- **The state bucket's name: `rust-leptos-axum-aws-tfstate-<account_id>`,
  computed at apply time and never committed.** `bootstrap` builds it from
  `data.aws_caller_identity`, so no literal appears in the configuration. The
  three upper layers need it as a backend literal, which a `backend` block
  cannot interpolate, so it lives in `infra/backend.hcl` — gitignored, passed
  with `terraform init -backend-config=../backend.hcl`, and templated by the
  committed `infra/backend.hcl.example`. `bootstrap` also publishes the name to
  SSM and renders the file's contents as an output, so the operator copies
  rather than derives it.

  Rejected: an account-id suffix written straight into the repository. It is the
  usual fix and it is one command shorter, but the user asked for the account id
  to stay out of the tree.

- **The Google client id and secret: Terraform variables with no default**, the
  secret marked `sensitive`. Supplied by a gitignored `*.auto.tfvars` or by
  `TF_VAR_google_client_secret`. Rejected: creating the pair as an SSM
  SecureString out of band and reading it with a data source. It would have
  matched the layers' own SSM contract and spared the operator from holding the
  value on every apply, but it adds a manual prerequisite step and the value
  lands in state either way.

- **The placeholder Lambda package: `data "archive_file"` over a committed stub**
  at `infra/api/placeholder/bootstrap`. It exists only so the first create has
  bytes to upload; `ignore_changes` on `filename` and `source_code_hash` means
  no later apply touches it. Rejected: committing a prebuilt zip, which puts a
  binary blob in the tree for the same effect.

- **The Lambda Web Adapter layer: `arn:aws:lambda:ap-northeast-1:753240598075:layer:LambdaAdapterLayerX86:28`.**
  Verified against the live Lambda API rather than recalled, which caught a
  wrong guess: the layer is named `LambdaAdapterLayerX86`, not
  `LambdaAdapterLayerX86_64`. Version 28 is adapter 1.0.1, published
  2026-05-28, `x86_64` only. The function is therefore `x86_64` — the arm64
  layer exists, but building `crates/server` for it would mean cross-compiling
  from this x86 devcontainer, which is a larger change than this work.

- **Version constraints:** `required_version = ">= 1.11.0"` (the floor for
  `use_lockfile`), `hashicorp/aws ~> 6.0` (6.57.1 is current), and
  `hashicorp/archive ~> 2.0` for the placeholder.

**Steps 3–6 — the four layers.** Written in dependency order and validating.
Three things emerged that the plan had not anticipated:

- **`data.aws_ssm_parameter.value` is marked sensitive by the AWS provider**, and
  a sensitive value cannot be used where Terraform needs a plain string — the
  CloudFront domain goes into a CORS allow-list and into Cognito callback URLs,
  both of which are printed in a plan. `nonsensitive()` unwraps it. Confirmed
  against the provider schema rather than assumed, because `nonsensitive()` is
  itself an error when applied to something that is not sensitive.

- **The Cognito user pool got `deletion_protection = "ACTIVE"` in addition to
  `prevent_destroy`.** Not named in `deployment.md`, and added anyway: DR-0005's
  whole case for a separate `identity` layer is that losing the pool is
  irreversible, and `prevent_destroy` only binds Terraform. This is an addition
  to the design rather than a divergence from it, and `deployment.md` now
  records it.

- **`/health` is left unauthenticated.** `deployment.md` says the HTTP API has a
  JWT authorizer and does not say what it covers. `ANY /api/{proxy+}` is behind
  it; `GET /health` is not, because a probe carries no token and the endpoint
  returns a constant.

**Found and reported, not worked around:** the SPA has no authentication code at
all. `crates/app/src/api.rs` calls `/api/greeting` with no `Authorization`
header, so against this configuration every API call would return 401. The
authorizer is what `deployment.md` specifies, so it is what was built; the gap
is frontend work and is now recorded as a constraint rather than designed
around.

**Steps 7–8 — plumbing.** `.gitignore` covers `.terraform/`, state, tfvars, and
the two deliberately-uncommitted backend files; `.terraform.lock.hcl` is
committed in each layer as the provider pin. The `justfile` gained `tf-init`,
`tf-fmt`, `tf-fmt-check`, `tf-validate`, `tf-plan` and `tf-apply`, with the
apply order in a comment above them since nothing enforces it.

**Steps 10–11 — the durable layer.** DR-0006 written, covering the three backend
questions and what was rejected for each. `deployment.md` updated with the real
layout, the init sequence, the routes table, the verified adapter layer ARN, and
six new constraints. `docs/design/index.md` gained DR-0006 and a correction to
the **ci** entry.

### 2026-08-04

**The first apply began, and `identity` failed on the hosted-UI domain.**
`bootstrap` and `delivery` went in. `just tf-init identity && just tf-apply identity`
created the user pool, the Google identity provider, the app client and three of
the four SSM parameters, then stopped:

```text
InvalidParameterException: Domain cannot contain reserved word: aws
```

Cognito reserves `aws`, `amazon` and `cognito` as substrings of a user-pool
domain prefix, so `rust-leptos-axum-aws-auth` was never creatable. This is the
first of the three apply-time-only risks named in Verification below to actually
fire, and not in the way that section predicted: the guess was a global-uniqueness
collision, and the real cause is a name rule no schema check can see.

Fixed by giving `hosted_ui_domain_prefix` the default `rust-leptos-axum-auth`.
`var.project` is untouched — the reserved-word rule binds this one hostname, not
the user pool, the buckets, the function or the SSM paths, all of which carry
`aws` without complaint. Renaming the project to restore uniformity was
considered and rejected; the reasoning, and the two other rejected shapes, are in
DR-0007.

The partial state is fine to re-apply over: everything created above is in state
and matches the configuration, so the re-run creates only the domain and the
`hosted_ui_domain` parameter.

**Carried out of Terraform:** the Google OAuth client's authorised redirect URI
still names the old prefix. It is a console value Terraform does not manage, so
Google sign-in stays broken until it is changed to
`https://rust-leptos-axum-auth.auth.ap-northeast-1.amazoncognito.com/oauth2/idpresponse`.

**All four layers applied.** `identity` re-ran clean and `api` went in. All ten
SSM parameters exist. The three apply-time-only risks named in Verification are
now settled: both bucket names took, and the domain prefix failed for the reason
above rather than for the collision that was predicted.

**The CloudFront domain answers `AccessDenied`, and the infrastructure is not at
fault.** Reported by the user as the first thing they saw in a browser. Checked
in this order, so the conclusion is not a guess:

- `aws s3 ls` on the origin bucket returns nothing. It is empty.
- The distribution is `Deployed`, its origin carries the OAC, `default_root_object`
  is `index.html`, and both custom error responses are in place.
- The bucket policy is the one in `main.tf`, scoped to this distribution's ARN.
- `curl -D-` on the domain returns `server: AmazonS3` and
  `x-cache: Error from cloudfront` — an origin error passed through, not a
  CloudFront one.

So: the root asks for `index.html`, which does not exist; the OAC principal holds
`s3:GetObject` and not `s3:ListBucket`, so S3 answers 403 rather than 404;
CloudFront falls back to `/index.html`, which 403s for the identical reason; the
fallback having failed, the origin's XML goes to the browser unchanged. Every
step is the configuration working as designed. What was missing was the artefact
deploy, which the plan's scope note put outside this work.

**Scope reopened by the user: add the deploy recipes, but do not run them.** The
`justfile` gained `deploy-web`, `deploy-api`, and a hidden `_ssm` helper, plus a
`project` variable mirroring `var.project`. The recipes read SSM and never open
state, so a deploy needs no `backend.hcl` and no `init` — which also means the
existing `tf-init` prerequisite does not spread to them.

Four things were found while writing them, none of which the design had:

- **`zip` is not in the devcontainer.** `unzip` is, installed for Terraform's own
  zip-only distribution, but `provided.al2023` takes a zip and `tar` cannot make
  one. Added to the Dockerfile. `deploy-api` therefore cannot run until the
  container is rebuilt, and could not be tested here; `deploy-web` is unaffected.

- **The native Lambda build survives on two versions of headroom, not on the
  matching architecture.** `deployment.md` justified `x86_64` by the devcontainer
  matching it, which is necessary and not sufficient: the image is Debian trixie
  with glibc 2.41 and `provided.al2023` ships 2.34. Measured rather than assumed —
  `objdump -T` over the release binary tops out at `GLIBC_2.34` exactly, so it
  runs. The margin is zero, and a dependency that raises it would fail at
  invocation with nothing catching it earlier. Recorded as a constraint with the
  check that reproduces it.

- **`dist/public/` cannot take the immutable cache header.** trunk hashes
  everything except the entry point *and* that directory, which it copies
  verbatim. `deployment.md` had only the two-way split — entry point against
  hashed assets — so a single sync with one header would have pinned a stale
  favicon in every browser cache for a year. It gets a short header and leans on
  the invalidation. This is the third case, and it is now in the design.

- **The wasm needs its content type stated.** Whether the AWS CLI guesses
  `application/wasm` could not be established from inside the container — the
  bundled Python's `mimetypes` is frozen into the binary and there is no
  `/etc/mime.types` — so it is set explicitly rather than left to chance. Checked
  what the cost of being wrong would be before deciding it mattered: the
  generated glue at `dist/app-*.js:1065` catches the `instantiateStreaming`
  failure and falls back to a non-streaming compile, so a wrong type is slow and
  noisy, not broken. One flag was cheaper than the fallback.

**Verified without applying anything:** `just --list` shows the three recipes
with `_ssm` hidden; `just --dry-run` renders both shebang recipes and `bash -n`
accepts each; `_ssm` expands to `/rust-leptos-axum-aws/delivery/spa_bucket`. The
recipes themselves have not been run — the user reserved that — so neither the
S3 layout they produce nor the zip step is proven.

Also noticed and left alone: `just --list` renders `tf-validate`'s description as
the last line of the comment block above it, mid-sentence. It predates this work.

**Steps 10–11 again, for the above.** `deployment.md` gained the recipe
descriptions, the four-pass upload table, and six constraints; the first-apply
note at the top now says what is and is not deployed. `index.md` corrected its
**ci** entry, which still claimed nothing had been applied. `workspace.md` says
its task table is not the whole `justfile`. No new Decision Record: everything
found above states what the system is, which is a Design Document's job, and
none of it is a decision with a rejected alternative worth preserving.

### 2026-08-05

**Both deploys were run, and the page failed in the way this log predicted.**
`just deploy-api` then `just deploy-web`, both by the user. The SPA loaded from
CloudFront and rendered `could not decode the response: expected value at line 1
column 1` where the greeting belongs.

The Verification section below had already named the cause a day earlier — a
bundle that calls a relative `/api/greeting` CloudFront does not route — and
`deployment.md` carried it as a written gap. What neither predicted is that the
failure is silent rather than loud: the request returns **200**, because the
SPA-fallback rule maps the origin's 403 to `/index.html`, so `api.rs` passes its
status check and fails only at `serde_json`. The symptom names the last step and
not the cause, which is worth more than the prediction was.

**Scope, stated plainly.** Running the deploys and fixing what they exposed
belongs to the artefact-deployment work, not to the Terraform setup this log was
opened for. It landed here because the request arrived here. The Retirement note
below already asked whether the remainder belonged in a new log; the answer is
that it did, and this entry is the record of it not having been split. The
frontend gap it left — the missing access token — is where the line is finally
drawn: that is a new log.

**The fix, in two parts.**

- **`API_BASE_URL`, read at compile time.** `crates/app/src/api.rs` reads it
  through `option_env!` into a constant and joins it to each absolute path;
  `deploy-web` resolves `/…/api/api_endpoint` from SSM and passes it to
  `trunk build`. This is what `deployment.md` had specified all along under
  "Configuring the SPA" and what nothing had implemented. Unset means the empty
  string, which leaves development relative and proxied, so one code path serves
  both. Two details worth keeping: the `$default` stage publishes its invoke URL
  **with a trailing slash**, stripped in the recipe and guarded again in the
  crate; and Cargo tracks variables reached through `option_env!`, so changing
  the endpoint rebuilds the crate instead of silently reusing a bundle built
  against another one. Verified by clearing the variable and watching `app`
  recompile.

- **`ANY /api/{proxy+}` split into one route per method.** Found while checking
  the fix, not looked for. An `ANY` route matches `OPTIONS`, which put the JWT
  authorizer in front of the CORS preflight: `curl -X OPTIONS` returned **401**
  where a preflight must be 2xx. The HTTP API's built-in preflight answer only
  covers an `OPTIONS` no route matches, which is the opposite of what
  `apigateway.tf` and `deployment.md` both asserted. The routes now come from
  `local.api_methods`, which `cors_configuration.allow_methods` also derives
  from, so the two lists cannot drift.

  This was invisible and would have stayed invisible: a `GET` with no
  `Authorization` header is a simple request and triggers no preflight, so the
  bug fires only once sign-in starts working — in the next unit of work, against
  code that would look like the culprit. Fixing it now cost one apply.

**Three errors found in `deployment.md`'s SSM table**, none of them today's
work and none of them reachable by `terraform validate`, which checks a layer
against the provider schema and never against a document. `cloudfront_domain` is
not read by the SPA build; `user_pool_id` is read by nothing; `app_client_id` is
read by `api` as the authorizer's audience, which the table omitted while naming
a consumer that does not exist yet. The table now says, per row, what reads the
value today and what will.

**Design documents updated:** `deployment.md` for the top note, the routes table,
the deploy recipe, "Configuring the SPA", the SSM table, and the two constraints
that were wrong — the CORS preflight claim and the `Authorization` gap.
`frontend.md` for the compile-time origin, which its "all API calls use relative
paths" constraint had contradicted. `index.md` for the **ci** entry. Awaiting the
user's confirmation, as the box below records.

~~**No new Decision Record.** Both changes implement what the durable layer
already specified — `deployment.md` named the compile-time variables, and the
preflight was an assertion in it that turned out to be false. Neither carried a
rejected alternative worth preserving, and the constraint each leaves behind is a
statement about what the system is, which is a Design Document's job. The
enumerated methods are the closest call: the trade-off is real, since a new
method now needs an infrastructure change where `ANY` needed none, but the
alternative is not a design option — it is broken.~~

**Superseded, later the same day, during close-out.** The judgment above tested
each change against *what the system is* and stopped there. Both fail a test it
did not apply: a Design Document cannot hold what the system is **not**, and both
changes rest on alternatives that were rejected and are recorded nowhere.

- **The compile-time configuration** was reached over a runtime `config.json`
  and over routing `/api` through CloudFront. `deployment.md` named the mechanism
  but never the alternatives, and the CloudFront one was sitting in its
  Constraints section — a Design Document explaining what the system is not,
  which is the signal to write a record. **DR-0008.**
- **The per-method routes** close the CORS gap DR-0001 explicitly left open, and
  nothing recorded how it was closed or that a `tower-http` CorsLayer in
  `crates/server` was the alternative — which fails for a reason worth keeping:
  behind an `ANY` route the service never receives the preflight to answer.
  **DR-0009.**

The claim above that "the alternative is not a design option — it is broken" is
right about `ANY` and wrong as a reason not to write a record: what is worth
preserving is not that `ANY` is broken but *why nobody would notice*, and that it
is `crates/server`, not API Gateway, that the obvious fix would have touched.

Both records are written. `deployment.md` and `frontend.md` now cite them and
carry the constraint rather than the reasoning.

## Verification

Run 2026-08-02, against the tree as committed.

- **`just tf-fmt-check`** — `terraform fmt -recursive -check infra`, exit 0. No
  file needed reformatting.
- **`just tf-validate`** — `terraform init -backend=false` then
  `terraform validate` in `bootstrap`, `delivery`, `identity`, `api`. All four
  report `Success! The configuration is valid.` No AWS credentials were used and
  no backend was contacted.
- **Read-through against `docs/design/deployment.md`.** All nine parameter paths
  in its Interfaces table are written by the layer it names, and the four
  cross-layer reads match the dependency graph. Every bullet in its Constraints
  section has a corresponding line in the configuration. `bootstrap/state_bucket`
  is a tenth parameter, added here and added to the table.
- **The adapter layer ARN was checked against the live Lambda API**, not
  recalled. This is the one fact in the configuration that was verified against
  AWS, and it corrected a wrong layer name.
- **`git check-ignore`** confirms `infra/backend.hcl`,
  `infra/bootstrap/backend.tf`, `*.tfstate`, `*.tfvars` and `.terraform/` are
  ignored, and that `.terraform.lock.hcl` is not.

`validate` is the ceiling and it is a low one. It proves the configuration is
internally consistent and matches the provider schema. It does not prove the
infrastructure works, that IAM permits what the layers need, or that the apply
order holds. Three names — the two buckets and the Cognito domain prefix — are
globally unique and can only fail at apply time. Nothing short of an apply
settles any of this.

Run 2026-08-04, after the applies and the deploy recipes.

- **The apply settled what `validate` could not.** All four layers are in the
  account and all ten SSM parameters resolve. Of the three globally unique names,
  two took and one failed — for a reserved word, not a collision (DR-0007).
- **The `AccessDenied` diagnosis was checked against the account**, not reasoned
  out: empty bucket, `Deployed` distribution with the expected origin, error
  responses and policy as configured, and a response carrying `server: AmazonS3`.
- **The Lambda's glibc ceiling was measured**, `objdump -T` over
  `target/release/server`, highest symbol `GLIBC_2.34`.
- **The deploy recipes were parsed and rendered, not run.** `just --list`,
  `just --dry-run` piped through `bash -n`, and `_ssm`'s expansion.

Still unproven, and only an artefact deploy settles it: that the SPA loads from
CloudFront, that the four-pass upload lands the headers and content types
intended, that `zip` is present after the rebuild, and that the Lambda binary
runs on `provided.al2023`. Two known gaps stand behind even a successful deploy —
the SPA sends no `Authorization` header and receives no compile-time
configuration, so it would call a relative `/api/greeting` that CloudFront does
not route.

Run 2026-08-05, after both deploys and the routing fix.

- **The deploy settled everything the run above left open.** `zip` is present,
  the Lambda binary runs on `provided.al2023` — `GET /health` returns `ok`
  through API Gateway — and the SPA loads from CloudFront.
- **The compile-time endpoint was traced into the artefact, not assumed.**
  `strings` over the bundle fetched back from the CloudFront domain finds the API
  Gateway origin; the same build with the variable unset does not contain it, and
  Cargo recompiles `app` between the two.
- **The preflight was checked from the SPA's own origin.** `OPTIONS
  /api/greeting` with `Origin:` the CloudFront domain returns 204 with
  `access-control-allow-origin`, `-methods`, `-headers` and `-max-age`. Before
  the route change the same request returned 401. A request from a foreign origin
  returns no `access-control-*` header at all.
- **The authorizer still covers what it should.** `GET /api/greeting` without a
  token is 401, `GET /health` is 200.
- **`just check`, `fmt-check`, `lint`, `tf-fmt-check`, `tf-validate`** all pass,
  and `terraform plan` on `api` was read before applying: two routes added, one
  destroyed, no other change.

What remains unproven is only what the next unit of work proves: that a request
carrying a real Cognito access token is authorised and returns the greeting. No
token has ever been minted, so the authorizer has been observed rejecting and
never accepting.

## Retirement

- [x] Design Documents updated — `deployment.md` reconciled with the
      configuration, with the deploy recipes, and with what the deploys proved;
      `frontend.md` with the compile-time API origin; `index.md` and
      `workspace.md` alongside them. Confirmed by the user on 2026-08-05, which
      is what this box records: `docs/README.md` makes a human the owner of any
      design-document overwrite, and being asked to write the update is not the
      same as having reviewed it.
- [x] Decision Records written (DR-0006, DR-0007, DR-0008, DR-0009)
- [x] Non-obvious knowledge preserved — rejected alternatives, pitfalls,
      constraints. The rejected backend alternatives are in DR-0006 and the
      hosted-UI naming in DR-0007; the adapter layer's real name, the
      `nonsensitive()` requirement, the unauthenticated SPA, the apply-time-only
      name collisions, the empty-bucket `AccessDenied`, the glibc ceiling, the
      unhashed `public/`, and the wasm content type are in `deployment.md`.
      The rejected runtime-configuration shapes are in DR-0008, and the
      preflight-behind-`ANY` trap with the CorsLayer alternative in DR-0009.
- [x] No durable document depends on this log — verified by grep over
      `docs/design/` and `docs/decisions/` for this file's name and its plan's.

Every box passes. Everything this log covers is built, applied, deployed and
checked, and the design documents are confirmed. The unit grew past the Terraform
setup it was opened for — the applies, the deploy recipes, running them, and the
routing fix all landed here rather than in logs of their own — and the 2026-08-05
Progress entry records that as a mis-scoping rather than as the shape the work
should have had.

The line is drawn here. The SPA obtains no access token, so the greeting still
renders a 401, and closing that is a new log: the hosted-UI redirect, the PKCE
exchange, somewhere to keep the token, and the header. It needs no infrastructure
change — `identity` already publishes `app_client_id` and `hosted_ui_domain`, and
the CORS preflight it will start triggering was fixed above.
