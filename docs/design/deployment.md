# Deployment

Updated: 2026-08-16

Note: all five layers have been applied and their twelve SSM parameters exist.
The API is deployed and `GET /health` returns `ok`. The container-image
packaging described below is applied and verified end to end against real
AWS. `crates/server` still answers `GET /api/dashboard` from hardcoded
values. The bundle on CloudFront was built without the two Cognito
variables, so it sends no token and `/api` calls are answered 401 there; one
`just deploy-web` replaces it with a build that signs in.

**DR-0028's move of Cognito verification into `crates/server` is applied and
verified end to end against real AWS.** The deployed function verifies
tokens itself; `aws_apigatewayv2_authorizer.cognito` and the parameter
mapping are gone. A real access token from the hosted UI was accepted and
attributed to its `sub`; the same token with a tampered signature was
refused; a request with no token was refused by the service itself,
confirmed in the Lambda's own CloudWatch log rather than API Gateway's; and
a CORS preflight with no token was answered by the HTTP API without
reaching the Lambda. An expired token was not exercised against real AWS —
a real access token is valid an hour — and is covered instead by
`cognito::tests::an_expired_token_is_refused` against the fixture key and a
fixed clock.

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
| API exposure | An API Gateway HTTP API; the function itself validates Cognito access tokens (DR-0028) |

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
  api/                       Lambda, HTTP API, CORS, IAM role, log groups,
                             the ECR repository the image lives in
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
| `/<project>/identity/user_pool_issuer` | `identity` | `api`, as the Lambda's `COGNITO_ISSUER` |
| `/<project>/identity/app_client_id` | `identity` | `api`, as the Lambda's `COGNITO_AUDIENCE`; the SPA build, as `COGNITO_CLIENT_ID` |
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

`crates/server` is an ordinary axum binary and is not modified for Lambda. It
is packaged as a container image, built by `infra/api/Dockerfile`, with the
Lambda Web Adapter turning the Lambda invocation into an HTTP request against
it — [DR-0026](../decisions/DR-0026-the-api-is-packaged-as-a-container-image.md).

- Both of the Dockerfile's stages are `public.ecr.aws/lambda/provided:al2023`.
  Building inside the runtime's own base, rather than in the devcontainer or on
  a generic Rust image, is what keeps the binary's glibc requirement from
  exceeding what the function actually ships — see the constraint below.
- The build stage installs a C toolchain with `microdnf` and a Rust toolchain
  with `rustup`, reading the version from `rust-toolchain.toml` — the same file
  the devcontainer reads — rather than naming the version a second time.
- The Lambda Web Adapter is copied in as an extension from its own published
  image, `public.ecr.aws/awsguru/aws-lambda-adapter:1.0.1`, to
  `/opt/extensions/lambda-adapter`.
- The image's `ENTRYPOINT` is `crates/server`'s own binary, copied to
  `/var/task/bootstrap` directly. That name and the adapter's own extension
  binary are unrelated despite the shared name.
- `AWS_LWA_PORT=3000`, matching the address `crates/server` binds, and
  `AWS_LWA_READINESS_CHECK_PATH=/health`, the endpoint the service already
  serves, are set on the function by Terraform (`infra/api/lambda.tf`), not in
  the Dockerfile.
- `TABLE_NAME` is set the same way, from the `data` layer's parameter, so the
  table name reaches the service without being written down twice.
- `COGNITO_ISSUER` and `COGNITO_AUDIENCE` are set from the `identity` layer's
  `user_pool_issuer`/`app_client_id` parameters — what
  `aws_apigatewayv2_authorizer.cognito`'s `jwt_configuration` used to hold,
  now read by the service directly (DR-0028). Both set is what selects
  `identity::Auth::Cognito` over the header-trusting `Auth::Mock` at startup;
  see `backend.md`.

**Routes.** The HTTP API declares one per method the SPA calls, plus the probe,
all through one integration — `crates/server` verifies the `Authorization`
header itself now, so nothing at this layer authorizes or maps anything:

| Route | Integration |
| --- | --- |
| `GET /api/{proxy+}` | `api` |
| `POST /api/{proxy+}` | `api` |
| `GET /health` | `api` |

`{proxy+}` means a new *endpoint* under `/api` in `crates/server` needs no change
to the infrastructure. A new *method* does: it goes in `local.api_methods` in
`infra/api/apigateway.tf`, which both this route set and the CORS
`allow_methods` list derive from, and which is the only place it lives — DR-0023
removed the second copy. The methods stay enumerated rather than covered by a
single `ANY` route so that the HTTP API answers CORS preflight itself without
proxying it to a function that has no `OPTIONS` handler — DR-0009, DR-0028, and
the CORS constraint below.

**The service verifies its own caller now; the route table is still verified
against real AWS, not locally.** `crates/server`'s `identity::Auth::Cognito`
does what `aws_apigatewayv2_authorizer.cognito` used to: fetch the pool's key
set once at startup, then check RS256, `iss`, `exp` and the audience —
satisfied by `client_id` for an access token or `aud` for an id token — on
every request, refusing anything that fails with no fallback. What remains
checked only against the deployed API, because it is behaviour AWS defines
rather than this repository's: the route table, and the preflight answered
ahead of the route table by `cors_configuration`. Both were reproduced
locally for a time, under DR-0021, and that was retracted — a local mirror
is a second telling of AWS's specification, maintained by hand, which drifts
silently (DR-0023). `just tf-validate` is as far as either can be checked in
advance; a fault in the route set or the preflight surfaces after an apply.

**`just dev-api-cognito` is where the Cognito configuration is checkable
before an apply**, the same role `just dev-gateway` played before DR-0028
retired it. It runs `crates/server` itself with `COGNITO_ISSUER`/
`COGNITO_AUDIENCE` resolved from the same two SSM parameters this layer reads
them from, so a wrong one is visible here rather than as a 401 after a
deploy indistinguishable from a broken sign-in.

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

**This recipe runs from either side.** The devcontainer reaches the host's
podman engine over a mounted socket
([DR-0027](../decisions/DR-0027-the-devcontainer-reaches-the-hosts-container-engine.md)),
so `docker build` / `push` / `update-function-code` behave the same whether
the shell running them is the devcontainer's or the host's. Deployment moving
to GitHub Actions is still future work, not made moot by this — it replaces a
person running either shell, not the recipe itself.

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

- **`just tf-apply api` comes before `just deploy-api`.** The two are on
  separate cadences (DR-0001), so one necessarily lands first, and applying
  first is the safe order: a Terraform change to the Lambda's configuration
  or the route table is inert until the next invocation, which the still-running
  binary answers exactly as before. **The one exception was the DR-0028
  transition itself**, which needed the reverse order once, for a reason that
  does not recur: removing the parameter mapping while the *old* binary still
  trusted it unconditionally would have attributed every request to the
  development owner silently. Deploying the new binary first was safe instead,
  because it falls back to `Auth::Mock`, which the still-active old mapping
  still satisfied correctly. That transition is recorded in DR-0028's
  Consequences; the standing apply-before-deploy rule above holds without
  exception now that `identity::Auth::Cognito` is the steady state, since
  every mismatch under it fails closed rather than open.

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
  API Gateway, whose HTTP APIs answer preflight themselves, ahead of the route
  table — so a preflight is not routed to the function and blocked for lacking a
  token. The allowed origin is the CloudFront domain. `crates/server` therefore
  stays free of a CORS layer, and local development stays single-origin through
  the trunk proxy — DR-0009.

  **That built-in answer only applies to an `OPTIONS` request no route matches**,
  which is why `/api/{proxy+}` is declared once per method instead of once as
  `ANY`. An `ANY` route matches `OPTIONS` too and would proxy every preflight to
  the function, which has no `OPTIONS` handler — DR-0028 confirmed this reasoning
  survives removing the JWT authorizer, since it never depended on one.

  `/api/*` is deliberately not routed through CloudFront as a second origin,
  which would have made the SPA single-origin and removed CORS entirely —
  DR-0008.

- **Reaching the API from a browser with real Cognito verification active
  means `dev-web-auth`.** `just dev-api-cognito` verifies a real token, and a
  `dev-web` bundle built without the two Cognito variables sends no
  `Authorization` header at all, so every `/api` call is a 401. That is the
  deployment constraint below, appearing locally as a consequence of what
  `identity::Auth::Cognito` does rather than as something arranged — DR-0028.

- **The service's audience check is matched against `client_id` for an access
  token and `aud` for an id token.** `COGNITO_AUDIENCE` holds the app client
  id, and which claim carries it depends on which token the SPA sent: a
  Cognito **access** token carries it as `client_id`, an **id** token as
  `aud`. Both are accepted, so both are a working configuration and neither is
  a way to tell them apart from the outside. `crates/app/src/api.rs` sends the
  access token. This is the detail `just dev-api-cognito` exists to make
  visible — it names which kind arrived and which claim satisfied the
  audience, on every accepted request — DR-0028.

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
  what makes its glibc requirement structural rather than assumed.** Building
  both of `infra/api/Dockerfile`'s stages on
  `public.ecr.aws/lambda/provided:al2023` closes the gap by construction:
  nothing links against a newer libc than the one the function ships, because
  none is present while linking —
  [DR-0026](../decisions/DR-0026-the-api-is-packaged-as-a-container-image.md).
  `objdump -T` over the binary inside the built image, filtered for `GLIBC_`,
  is the check, and tops out at `GLIBC_2.34`.

- **`docker` (or a docker-CLI-compatible engine) is available on both sides.**
  The devcontainer carries the client and reaches the host's podman engine
  over a mounted socket — [DR-0027](../decisions/DR-0027-the-devcontainer-reaches-the-hosts-container-engine.md)
  — so `deploy-api` is not a host-only recipe; see "Deploying artefacts"
  above.

- **The project name is spelled out in the `justfile` as well as in every
  layer's `variables.tf`.** The deploy recipes address SSM paths rooted at that
  name, and `just` has no way to read a Terraform variable. The duplication is
  deliberate — it is what keeps a deploy free of Terraform state — and the two
  are kept in step by hand.

- **The Lambda's `image_uri` is under `ignore_changes`.** Without it, every
  `terraform apply` would revert the function to whatever `:latest` resolves to
  in the ECR repository at that moment — always the image `deploy-api` most
  recently pushed, since there is no separate placeholder image.

- **The ECR repository has to hold an image before `aws_lambda_function.api`
  can be created with `package_type = "Image"`.** This has no bearing on the
  ordinary apply-then-deploy cycle, where an image already exists from the
  previous deploy — it matters only when the function does not exist yet.
  Applying `infra/api` with the ECR repository but without the function first,
  running `deploy-api` once the repository exists, then applying the function
  is the sequence; there is no committed placeholder image to apply against
  instead. This is also the one apply in this document that is not the
  zero-downtime kind the rest of its ordering constraints describe, since
  `package_type` cannot change on an existing function.

- **A bundle built without the two Cognito variables cannot call the API.** The
  SPA obtains a token and attaches it (DR-0010), but only when
  `COGNITO_CLIENT_ID` and `COGNITO_HOSTED_UI_DOMAIN` were set at build time.
  Unset, it sends no header — correct against `just dev-api`, where
  `identity::Auth::Mock` is running and reads no token at all, and a 401 on
  every `/api` call once deployed, where `identity::Auth::Cognito` refuses a
  request with none. `just deploy-web` sets both, so the way to produce that
  bundle is to build it by some other route.

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
