# Deployment

Updated: 2026-08-15

Note: all five layers have been applied and their twelve SSM parameters exist.
The API is deployed and `GET /health` returns `ok`. The table is empty and
nothing reads or writes it yet — `crates/server` still answers
`GET /api/dashboard` from hardcoded values. The bundle on CloudFront was built
without the two Cognito variables, so it sends no token and `/api` calls are
answered 401 there; one `just deploy-web` replaces it with a build that signs
in.

Note: the `AuthContext` parameter mapping below is configured and **not yet
applied**, and the deployed function is still the binary that predates it. The
two are consistent as they stand. Bringing them forward has a required order —
see the constraint on it below, which is the one place in this document where
doing the two steps the wrong way round is worse than doing neither.

Note: this document describes the container-image packaging below as the
target shape. Neither `infra/api` nor `just deploy-api` has been applied in
that shape yet — the deployed function is still the zip plus Lambda-layer form.
`docs/work/2026-08-10-api-artefact-packaging.md` is the record of the
migration and its outstanding steps.

## Purpose

How this project runs on AWS, and how the infrastructure that runs it is
described and applied.

There is one environment. Development happens locally — `trunk serve` for the
SPA and `cargo run -p server` for the API — so everything below describes
production, and the only axis the configuration is divided along is blast
radius (DR-0005).

The runtime shape follows from DR-0001: the frontend is static files on a CDN,
the API is a separate service, and the two are released independently.

## Structure

### The running system

| Piece | Runs on |
| --- | --- |
| The Leptos SPA (`dist/`) | A private S3 bucket, served through CloudFront with Origin Access Control |
| Authentication | A Cognito User Pool with Google as an identity provider, Authorization Code Flow with PKCE |
| Application data | One on-demand DynamoDB table, keyed by owner — see [persistence.md](persistence.md) |
| The axum API (`crates/server`) | AWS Lambda on `provided.al2023`, packaged as a container image with the AWS Lambda Web Adapter built in |
| API exposure | An API Gateway HTTP API with a JWT authorizer validating Cognito access tokens |

### The configuration

Terraform (DR-0004), split into five root modules, each with its own state file
in the bootstrap bucket and its own apply (DR-0005):

```text
infra/
  backend.hcl.example        template for the uncommitted backend.hcl — DR-0006
  bootstrap/                 the S3 bucket holding every other layer's state
    backend.tf.example       the migration snippet, copied into place once
  delivery/                  S3 origin bucket, CloudFront, OAC, cache behaviour,
                             SPA fallback
  identity/                  Cognito User Pool, Google IdP, hosted-UI domain,
                             app client
  data/                      the DynamoDB table — DR-0015
  api/                       Lambda, HTTP API, JWT authorizer, CORS, IAM role,
                             log groups, the ECR repository the image lives in
    Dockerfile               builds crates/server into the image the function runs
```

Every layer holds `versions.tf`, `variables.tf`, `outputs.tf` and its resources;
`api` splits those across `iam.tf`, `lambda.tf` and `apigateway.tf`. Each
layer's `.terraform.lock.hcl` is committed — it is the provider pin.

Names all derive from one `project` variable, `rust-leptos-axum-aws`, which is
also the root of the SSM paths below. The two bucket names carry the account id
as a uniqueness suffix, assembled at apply time from `aws_caller_identity`. The
Cognito hosted-UI domain prefix is the single exception and is `rust-leptos-axum-auth`,
for the reason in the Constraints section.

Dependencies run one way:

```text
bootstrap
   │
   ├── data ── table_name, table_arn ─────────────────────────────────▶
   ▼                                                                  │
delivery ── cloudfront_domain ──▶ identity ── issuer, client_id ──▶ api
   └──────── cloudfront_domain (CORS allowed origin) ─────────────────▶
```

So a first create is `bootstrap`, `delivery`, `identity`, `data`, `api`, applied
in that order, and a teardown is the reverse. `delivery` sits above nothing but
`bootstrap` because the SPA is static — it learns the API URL and the Cognito
client id at build time, not through Terraform. `data` sits above nothing but
`bootstrap` either, since a table needs no value from any other layer; its place
fourth in the sequence is convention, and the only ordering that is real is that
it precedes `api`.

`bootstrap` is applied once against a local backend, then its own state is
migrated into the bucket it created. The bucket has versioning, encryption, a
public-access block, and `prevent_destroy`. State is locked by S3's native lock
file, not a DynamoDB table (DR-0006) — the table the `data` layer owns is
application data and has no part in state locking.

Each layer's `backend "s3"` block declares only its `key`. The bucket name is
not in the repository, because it carries the AWS account id, so the remaining
backend settings come from `infra/backend.hcl` — gitignored, templated by
`infra/backend.hcl.example`, and rendered as a `bootstrap` output (DR-0006).
The whole first-create sequence is:

```sh
terraform -chdir=infra/bootstrap init
terraform -chdir=infra/bootstrap apply
terraform -chdir=infra/bootstrap output -raw backend_hcl > infra/backend.hcl
cp infra/bootstrap/backend.tf.example infra/bootstrap/backend.tf
terraform -chdir=infra/bootstrap init -migrate-state -backend-config=../backend.hcl

just tf-init delivery && just tf-apply delivery
just tf-init identity && just tf-apply identity   # needs the Google credentials
just tf-init data     && just tf-apply data
just tf-init api      && just tf-apply api
```

`just tf-validate` schema-checks all five layers without credentials or a
backend, which is as far as anything can be verified before an apply.

## Interfaces

### What each layer publishes

Layers exchange values through SSM Parameter Store, never by reading each
other's state (DR-0005). A layer writes `aws_ssm_parameter` resources and reads
what it needs with the `aws_ssm_parameter` data source.

| Parameter | Written by | Read by |
| --- | --- | --- |
| `/<project>/bootstrap/state_bucket` | `bootstrap` | operators, so the bucket is findable without reading state |
| `/<project>/delivery/cloudfront_domain` | `delivery` | `identity`, for the callback and logout URLs; `api`, for the CORS allowed origin |
| `/<project>/delivery/cloudfront_distribution_id` | `delivery` | `just deploy-web` |
| `/<project>/delivery/spa_bucket` | `delivery` | `just deploy-web` |
| `/<project>/identity/user_pool_id` | `identity` | operators, so the pool is addressable without reading state |
| `/<project>/identity/user_pool_issuer` | `identity` | `api`, for the JWT authorizer |
| `/<project>/identity/app_client_id` | `identity` | `api`, as the authorizer's audience; the SPA build, as `COGNITO_CLIENT_ID` |
| `/<project>/identity/hosted_ui_domain` | `identity` | the SPA build, as `COGNITO_HOSTED_UI_DOMAIN` |
| `/<project>/data/table_name` | `data` | `api`, as the Lambda's `TABLE_NAME` |
| `/<project>/data/table_arn` | `data` | `api`, to scope the Lambda's IAM policy |
| `/<project>/api/api_endpoint` | `api` | `just deploy-web`, as `API_BASE_URL` |
| `/<project>/api/lambda_function_name` | `api` | `just deploy-api` |
| `/<project>/api/ecr_repository_url` | `api` | `just deploy-api`, as where the image is built and pushed |

The build and the deploy commands are the second consumer of these parameters,
which is why they are published rather than passed through state. Two are read
by nothing automated: they are published so that an operator can find the bucket
and the pool without opening state, which is the same reason as the rest.

`just dev-web-auth` is the third consumer, reading the two `identity` parameters
so the sign-in flow can be exercised against `trunk serve`.

### The API's runtime shape

`crates/server` is an ordinary axum binary and is not modified for Lambda. The
Lambda Web Adapter turns the Lambda invocation into an HTTP request against it.
It is packaged as a container image, built by `infra/api/Dockerfile`, rather
than as a zip plus an attached Lambda layer — the earlier form, and why it
changed, is `docs/work/2026-08-10-api-artefact-packaging.md` and its Decision
Record.

- Both of the Dockerfile's stages are `public.ecr.aws/lambda/provided:al2023`.
  Building inside the runtime's own base, rather than in the devcontainer or on
  the host's Rust image, is what keeps the binary's glibc requirement from
  exceeding what the function actually ships — see the constraint below.
- The build stage installs a C toolchain with `microdnf` and a Rust toolchain
  with `rustup`, reading the version from `rust-toolchain.toml` — the same file
  the devcontainer reads — rather than naming the version a second time.
- The Lambda Web Adapter is copied in as an extension from its own published
  image, `public.ecr.aws/awsguru/aws-lambda-adapter:1.0.1`, to
  `/opt/extensions/lambda-adapter`. This is the same adapter version the
  earlier layer form published; only how it attaches to the function changed.
- The image's `ENTRYPOINT` is `crates/server`'s own binary, copied to
  `/var/task/bootstrap`, directly. There is no `AWS_LAMBDA_EXEC_WRAPPER`: that
  setting made the adapter layer's `/opt/bootstrap` the entry point under the
  zip form, and has no role once the adapter is an extension the runtime starts
  on its own and the image's `ENTRYPOINT` is the service.
- `AWS_LWA_PORT=3000`, matching the address `crates/server` binds, and
  `AWS_LWA_READINESS_CHECK_PATH=/health`, the endpoint the service already
  serves, are set on the function by Terraform (`infra/api/lambda.tf`), not in
  the Dockerfile — the same as under the zip form.
- `TABLE_NAME` is set the same way, from the `data` layer's parameter, so the
  table name reaches the service without being written down twice.
  `crates/server` does not read it yet.

The `/var/task/bootstrap` name and the adapter's own extension binary are
unrelated, despite both being named `bootstrap` under the zip form this
replaces, where the adapter's layer separately provided `/opt/bootstrap`.

**Routes.** The HTTP API declares one per method the SPA calls, plus the probe:

| Route | Authorization | Integration |
| --- | --- | --- |
| `GET /api/{proxy+}` | JWT authorizer, so every call the SPA makes carries a Cognito access token | `api` |
| `POST /api/{proxy+}` | the same | `api` |
| `GET /health` | none — a probe has no token, and the endpoint returns a constant | `health` |

**The edge produces the service's `AuthContext`.** `crates/server` reads two
headers and knows nothing about API Gateway (`backend.md`, DR-0024). This layer is
where the conversion happens, by request parameter mapping on the `api`
integration:

```hcl
"overwrite:header.x-auth-subject" = "$context.authorizer.claims.sub"
"overwrite:header.x-auth-edge"    = "apigateway"
```

Mapping is AWS's own mechanism, so nothing here re-implements AWS behaviour
(DR-0023), and because the subject is one value API Gateway resolves, no map of
claims is forwarded or parsed anywhere.

`overwrite:` is load-bearing: the service trusts these headers only because the
edge replaces whatever the caller sent, and `append:` would end that quietly.

**Two integrations to one function, which is the only reason there are two.**
Mapping is an attribute of the integration, not of the route, and `/health` is
routed outside the authorizer where `$context.authorizer.claims.sub` cannot
resolve. API Gateway skips a mapping whose source does not resolve, so a shared
integration would let a caller's own `x-auth-*` headers through on the probe. The
`health` integration carries `remove:` for both instead — DR-0025.

`{proxy+}` means a new *endpoint* under `/api` in `crates/server` needs no change
to the infrastructure. A new *method* does: it goes in `local.api_methods` in
`infra/api/apigateway.tf`, which both this route set and the CORS
`allow_methods` list derive from, and which is the only place it lives — DR-0023
removed the second copy. The methods are enumerated rather than covered by a single
`ANY` route so that the HTTP API answers CORS preflight itself — DR-0009, and the
CORS constraint below.

**The edge is verified here, not locally.** Everything this section describes
between the browser and the service — the route table, the preflight answered
ahead of the authorizer, the 401 for a request with no token, and the parameter
mapping that produces the `AuthContext` — is checked by exercising the deployed
API, because it is behaviour AWS defines. It was reproduced locally for a time,
under DR-0021, and that was retracted: a local mirror of this section is a second
telling of AWS's specification, maintained by hand, which drifts silently and can
never be authoritative about the thing it imitates (DR-0023). So a fault in the
route set or the preflight surfaces after an apply rather than before one, and
`just tf-validate` is as far as anything can be checked in advance.

The mapping is the newest member of that list and the one worth naming: `just
tf-validate` schema-checks it and cannot evaluate a `$context` expression. The
check after an apply is that a `/api` call is attributed to the token's `sub`
rather than to the development owner, and that `GET /health` still answers `ok`.

**The authorizer's configuration is the exception, and is checkable before an
apply.** `just dev-gateway` runs a thin adapter that does nothing but reach the
authorizer's verdict: it resolves `issuer` and `audience` from the same two SSM
parameters this layer reads them from, fetches the pool's keys from
`{issuer}/.well-known/jwks.json`, and accepts or refuses a real token the way
`aws_apigatewayv2_authorizer.cognito` would — DR-0022. A refusal answers exactly
what the deployed authorizer answers and prints the reason on its own terminal,
which is the distinction that does not exist in production, where every one of
these faults is an indistinguishable 401. This one is worth keeping where the
rest was not: `jwt_configuration` is four lines this repository owns, its faults
are otherwise indistinguishable from a broken sign-in, and the check uses the
real pool's real keys rather than a description of them — DR-0023.

### Deploying artefacts

Terraform owns the bucket and the function; it does not own their contents
(DR-0005). Artefacts are pushed separately, on their own cadence, by two `just`
recipes. Both resolve every name they need — bucket, distribution id, function
name — from the SSM parameters above rather than from Terraform state, so a
deploy needs neither `infra/backend.hcl` nor a `terraform init`. The two are
independent and have no ordering between them, which is DR-0001 showing up in
the task runner; there is deliberately no recipe that runs both.

**Frontend — `just deploy-web`.** Resolve the three build variables from SSM and
hand them to `trunk build --release` — see "Configuring the SPA" below — then
four uploads and an invalidation. The uploads are split because `dist/` is not
homogeneous:

| Pass | Contents | `Cache-Control` |
| --- | --- | --- |
| 1 | every hashed asset except `*.wasm` | `public, max-age=31536000, immutable` |
| 2 | `*.wasm` | as pass 1, plus an explicit `application/wasm` type |
| 3 | `public/` | `public, max-age=300` |
| 4 | `index.html` | `no-cache` |

The order is the load-bearing part: every hashed asset is in place before
`index.html`, so the entry point never references a file that is not there yet.
Each pass carries `--delete`, which retires the previous build's files in the
same sweep; keys matched by a pass's `--exclude` are exempt from its deletes as
well as from its uploads, so the passes do not clobber one another.

The invalidation is a wildcard. Only `public/` strictly needs one — hashed
assets change name, and `index.html` is on `Managed-CachingDisabled` — but
invalidations are free at this volume and a wildcard survives a change to the
cache behaviours.

**API — `just deploy-api`.** Build the image from `infra/api/Dockerfile`, push it
to the `api` layer's ECR repository under the `latest` tag, then
`aws lambda update-function-code --image-uri`, then
`aws lambda wait function-updated`, since the update call returns before the new
code is live. No Terraform run is involved.

**This recipe runs on the host, not in the devcontainer.** The devcontainer has
no container engine, and deployment is intended to move to GitHub Actions later
rather than have one added to it —
`docs/work/2026-08-10-api-artefact-packaging.md`. `just deploy-web` and
`just tf-*` are unaffected and still run from either side; this is the one
recipe in this document a devcontainer shell cannot complete.

### Configuring the SPA

The SPA needs the API endpoint, the Cognito app client id and the hosted-UI
domain. Each is read from SSM and passed to `trunk build` as an environment
variable, which the crate reads at compile time rather than fetching at runtime —
DR-0008. No hostname is written into the source — the constraint in
`docs/design/frontend.md` holds — and because the values land inside a
content-hashed bundle, a configuration change invalidates its own cache entry.

| Variable | Source | Read by |
| --- | --- | --- |
| `API_BASE_URL` | `/<project>/api/api_endpoint`, trailing slash stripped | `crates/app/src/api.rs` |
| `COGNITO_CLIENT_ID` | `/<project>/identity/app_client_id` | `crates/app/src/auth.rs` |
| `COGNITO_HOSTED_UI_DOMAIN` | `/<project>/identity/hosted_ui_domain` | `crates/app/src/auth.rs` |

All three are set by `just deploy-web`. There is deliberately no fourth for the
redirect URI: `auth.rs` computes it from `window.location.origin`, so it is
whatever origin serves the page and cannot drift from the callback URLs the app
client registers — DR-0010.

Every one is unset in development, and every unset value means something
workable. `API_BASE_URL` becomes the empty string, which leaves each call
relative and therefore served by the trunk proxy — the default that makes the
same code single-origin locally and cross-origin in production. An empty
`COGNITO_*` pair disables sign-in: no control, no `Authorization` header, and the
local API answers anyway because it validates nothing. `just dev-web-auth`
supplies the two real values when the flow itself is being worked on.

Cargo tracks variables reached through `option_env!`, so changing one rebuilds
the crate rather than silently reusing a bundle built against a different
endpoint or a different user pool.

An unset variable also leaves nothing behind: a bundle built without the two
Cognito variables contains no hosted-UI hostname and no `/oauth2/` path at all,
so a `grep` over `dist/*.wasm` tells a configured build from an unconfigured one
without running either.

## Constraints

- **`just tf-apply api` comes before `just deploy-api`, and the reverse order
  misattributes every request.** The mapping and the binary that reads it are
  two artefacts on two cadences (DR-0001), so one necessarily lands first.
  Applying first is safe: the edge sets two headers the old binary ignores.
  Deploying first is not: the new binary would find no `x-auth-edge` on any
  request, read that as "no edge spoke", and attribute every user's writes to the
  development owner — silently, with a 200, in exactly the way DR-0024 and
  DR-0025 exist to prevent. This is the one deployment ordering in this project
  that is unsafe rather than merely inconvenient, and nothing enforces it.

- **CloudFront maps both 403 and 404 to `/index.html` with status 200.** Deep
  links are router paths, not files, so without this every reload on a non-root
  route fails — DR-0001. 403 matters as much as 404 here: the origin bucket is
  private behind Origin Access Control, which grants `s3:GetObject` and not
  `s3:ListBucket`, so a missing key returns `AccessDenied` rather than
  `NoSuchKey`.

  The same rule makes an *empty* bucket look like a permissions fault. Before the
  first `just deploy-web`, the distribution root asks for `index.html`, gets 403,
  falls back to `/index.html`, gets 403 again, and passes S3's `AccessDenied` XML
  through to the browser. Nothing is misconfigured when that happens; nothing has
  been deployed.

- **The SPA and the API are separate origins, and CORS is configured on the
  HTTP API rather than in `crates/server`.** DR-0001 records the missing CORS
  layer in the service as a gap deployment has to close; deployment closes it at
  API Gateway, whose HTTP APIs answer preflight themselves, ahead of any
  authorizer — so a preflight is not rejected for lacking a token. The allowed
  origin is the CloudFront domain. `crates/server` therefore stays free of a CORS
  layer, and local development stays single-origin through the trunk proxy —
  DR-0009.

  **That built-in answer only applies to an `OPTIONS` request no route matches**,
  which is why `/api/{proxy+}` is declared once per method instead of once as
  `ANY`. An `ANY` route matches `OPTIONS` too and puts the JWT authorizer in
  front of the preflight, which is answered 401 and blocks the request it
  precedes.

  `/api/*` is deliberately not routed through CloudFront as a second origin,
  which would have made the SPA single-origin and removed CORS entirely —
  DR-0008.

- **Reaching the API from a browser through `just dev-gateway` means
  `dev-web-auth`.** The adapter verifies a real token, and a `dev-web` bundle
  built without the two Cognito variables sends no `Authorization` header at all,
  so every `/api` call is a 401. That is the deployment constraint below,
  appearing locally as a consequence of what the adapter does rather than as
  something arranged — DR-0022.

- **The authorizer's `audience` is matched against `client_id` for an access
  token and `aud` for an id token.** `jwt_configuration.audience` holds the app
  client id, and which claim carries it depends on which token the SPA sent: a
  Cognito **access** token carries it as `client_id`, an **id** token as `aud`.
  API Gateway accepts either, so both are a working configuration and neither is
  a way to tell them apart from the outside. `crates/app/src/api.rs` sends the
  access token. This is the detail `just dev-gateway` exists to make visible — it
  names which kind arrived and which claim satisfied the audience, on every
  accepted request — DR-0022.

- **One Cognito app client serves production and local development alike**, so
  its callback and logout URLs list the CloudFront domain *and*
  `http://localhost:8080`, the address in `Trunk.toml`. This is a property of
  the `identity` layer, not a second environment sneaking in.

- **The app client is public and has no client secret.** A secret cannot be kept
  in a WASM bundle the browser downloads; PKCE is what replaces it.

- **`index.html` is served with a short or no-cache header; hashed assets get a
  long one.** trunk emits content-hashed filenames for everything but the entry
  point, so caching `index.html` serves a stale bundle reference to every
  returning visitor. The CloudFront cache policies say this to CloudFront; the
  `Cache-Control` headers set at upload say it to the browser, which the cache
  policies do not reach.

- **`dist/public/` is the third case, and it is neither.** trunk copies that
  directory verbatim (`copy-dir` in `index.html`), so `public/favicon.svg` keeps
  its name across every build. The immutable header the hashed assets get would
  pin a stale copy in browser caches indefinitely, so it takes a short one and
  relies on the invalidation instead. Anything added to `public/` inherits that
  property.

- **The wasm bundle's content type is set explicitly at upload.** Left to the
  CLI's guess it may arrive as something other than `application/wasm`, and
  `WebAssembly.instantiateStreaming` refuses a response whose type is not that.
  This is not fatal — the wasm-bindgen glue catches the failure and falls back to
  a non-streaming compile — but the fallback is slower and warns in every
  visitor's console, and one flag avoids it.

- **The Lambda binary is built inside the runtime's own base image, which is
  what makes its glibc requirement structural rather than assumed.** A binary
  built in the devcontainer or on the host's own Rust image once required glibc
  symbols newer than `provided.al2023` ships, and failed only at invocation,
  with no schema check or test seeing it first —
  `docs/work/2026-08-10-api-artefact-packaging.md` is the record of that
  failure and why packaging moved to a container image over it. Building both
  of `infra/api/Dockerfile`'s stages on `public.ecr.aws/lambda/provided:al2023`
  closes the gap by construction: nothing links against a newer libc than the
  one the function ships, because none is present while linking. `objdump -T`
  over the binary inside the built image, filtered for `GLIBC_`, is still the
  check, and still tops out at `GLIBC_2.34`.

- **`docker` (or a docker-CLI-compatible engine) is a host dependency of
  `deploy-api`, not a devcontainer one.** The devcontainer has no container
  engine, so the image is built and pushed from the host — see "Deploying
  artefacts" above.

- **The project name is spelled out in the `justfile` as well as in every
  layer's `variables.tf`.** The deploy recipes address SSM paths rooted at that
  name, and `just` has no way to read a Terraform variable. The duplication is
  deliberate — it is what keeps a deploy free of Terraform state — and the two
  are kept in step by hand.

- **The Lambda's `image_uri` is under `ignore_changes`.** Without it, every
  `terraform apply` would revert the function to whatever `:latest` resolves to
  in the ECR repository at that moment, which is only ever the image
  `deploy-api` most recently pushed — not a distinct placeholder, since a
  container image has no equivalent of a bytes-free stub the way the zip form's
  `archive_file` did.

- **The ECR repository has to hold an image before `aws_lambda_function.api`
  can be created with `package_type = "Image"`.** This has no bearing on the
  ordinary apply-then-deploy cycle, where an image already exists from the
  previous deploy — it matters once, migrating from the zip form or standing
  the layer up from nothing. Applying `infra/api` with the ECR repository but
  without the function first, running `deploy-api` once the repository exists,
  then applying the function is the sequence; there is no committed placeholder
  image to apply against instead. Migrating an already-running function also
  replaces it — `package_type` cannot change in place — so this apply is not
  the zero-downtime kind the rest of this document's ordering constraints
  describe.

- **A bundle built without the two Cognito variables cannot call the API.** The
  SPA obtains a token and attaches it (DR-0010), but only when
  `COGNITO_CLIENT_ID` and `COGNITO_HOSTED_UI_DOMAIN` were set at build time.
  Unset, it sends no header — correct locally, where the axum server validates
  nothing, and a 401 on every `/api` call once deployed. `just deploy-web` sets
  both, so the way to produce that bundle is to build it by some other route.

- **One app client, so a token obtained locally is one the deployed API
  accepts.** Both origins are registered on the same client and the authorizer's
  audience is that client's id. This is what allows the deployed API to be tested
  with a token from `just dev-web-auth`, and it is a consequence of the
  single-client decision above rather than a separate arrangement.

- **The Lambda is `x86_64`.** It matches the architecture `infra/api/Dockerfile`
  builds on natively. Moving to arm64 means building the image on an arm64 host
  (or cross-building it) and switching the Dockerfile's adapter `COPY` to the
  adapter image's arm64 tag, which is why the architecture is a variable rather
  than a constant.

- **The Cognito user pool carries `deletion_protection` as well as
  `prevent_destroy`.** DR-0005 rates its loss as the only irreversible one in the
  stack, and `prevent_destroy` stops only Terraform. Deletion protection stops
  the console and the CLI too, and has to be turned off explicitly before the
  pool can ever be removed.

- **The Cognito hosted-UI domain prefix is the one name that does not derive
  from `project`.** Cognito rejects a prefix containing `aws`, `amazon` or
  `cognito`, and `project` contains `aws`, so the prefix is
  `rust-leptos-axum-auth` — DR-0007. The rule binds this name alone; the user
  pool, the buckets, the function and every SSM path carry `aws` freely.

- **Three names are globally unique and can only fail at apply time.** The two
  bucket names take the account id as a suffix and are unlikely to collide; the
  hosted-UI domain prefix has no such suffix, because it is a public hostname
  and the suffix would be the account id, so it is a plausible first-apply
  failure. It is a variable, so a collision costs one override.

- **`crates/server` binds `127.0.0.1:3000` as a constant.** The Lambda
  configuration accommodates it with `AWS_LWA_PORT`; nothing checks that the two
  agree. If the service ever learns to read a port from the environment, that
  variable goes away and this document is what has to change with it.

- **The state bucket carries `prevent_destroy` and versioning.**
  `prevent_destroy` stops Terraform, not the console; versioning is what makes
  the remaining case recoverable.

- **`infra/backend.hcl` is not committed and every layer's `init` needs it.**
  It carries the state bucket name, which carries the AWS account id (DR-0006).
  A fresh clone therefore cannot plan anything until the file is regenerated
  from `bootstrap`'s output or copied across.

- **The Google client id and secret are never written into the repository.**
  They are Terraform variables with no default, supplied by a gitignored
  `*.auto.tfvars` or by `TF_VAR_google_client_id` and
  `TF_VAR_google_client_secret`. The Google OAuth client's authorised redirect
  URI is the hosted UI's `/oauth2/idpresponse`, which is a value the console
  needs and Terraform does not manage — so it is derived from the domain prefix
  above and has to be re-entered by hand whenever that prefix moves.

- **Nothing enforces the apply order between layers.** Terraform sees five
  unrelated root modules. The order in Structure above is maintained by this
  document, and repeated as a comment above the `tf-` recipes in the
  `justfile`.

- **The DynamoDB table is guarded as heavily as the user pool.** DR-0005 rated
  the pool's loss as the only irreversible one in the stack; the table now joins
  it, so it carries `prevent_destroy`, `deletion_protection_enabled`, and
  point-in-time recovery for the application-level mistake neither guard sees.
  What it stores, and how, is [persistence.md](persistence.md).

- **Everything lives in one region.** No cross-region resource exists today. A
  custom domain would introduce one, since CloudFront requires its ACM
  certificate in `us-east-1`.
