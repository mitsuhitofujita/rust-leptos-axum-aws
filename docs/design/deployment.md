# Deployment

Updated: 2026-08-02

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
| The axum API (`crates/server`) | AWS Lambda on `provided.al2023`, fronted by the AWS Lambda Web Adapter |
| API exposure | An API Gateway HTTP API with a JWT authorizer validating Cognito access tokens |

### The configuration

Terraform (DR-0004), split into four root modules, each with its own state file
in the bootstrap bucket and its own apply (DR-0005):

```text
infra/
  bootstrap/    the S3 bucket holding every other layer's state
  delivery/     S3 origin bucket, CloudFront, OAC, cache behaviour, SPA fallback
  identity/     Cognito User Pool, Google IdP, hosted-UI domain, app client
  api/          Lambda, HTTP API, JWT authorizer, CORS, IAM role, log groups
```

Dependencies run one way:

```text
bootstrap
   │
   ▼
delivery ── cloudfront_domain ──▶ identity ── issuer, client_id ──▶ api
   └──────── cloudfront_domain (CORS allowed origin) ─────────────────▶
```

So a first create is `bootstrap`, `delivery`, `identity`, `api`, applied in that
order, and a teardown is the reverse. `delivery` sits above nothing but
`bootstrap` because the SPA is static — it learns the API URL and the Cognito
client id at build time, not through Terraform.

`bootstrap` is applied once against a local backend, then its own state is
migrated into the bucket it created. The bucket has versioning, encryption, a
public-access block, and `prevent_destroy`.

## Interfaces

### What each layer publishes

Layers exchange values through SSM Parameter Store, never by reading each
other's state (DR-0005). A layer writes `aws_ssm_parameter` resources and reads
what it needs with the `aws_ssm_parameter` data source.

| Parameter | Written by | Read by |
| --- | --- | --- |
| `/<project>/delivery/cloudfront_domain` | `delivery` | `identity`, `api`, the SPA build |
| `/<project>/delivery/cloudfront_distribution_id` | `delivery` | the deploy |
| `/<project>/delivery/spa_bucket` | `delivery` | the deploy |
| `/<project>/identity/user_pool_id` | `identity` | `api` |
| `/<project>/identity/user_pool_issuer` | `identity` | `api` |
| `/<project>/identity/app_client_id` | `identity` | the SPA build |
| `/<project>/identity/hosted_ui_domain` | `identity` | the SPA build |
| `/<project>/api/api_endpoint` | `api` | the SPA build |
| `/<project>/api/lambda_function_name` | `api` | the deploy |

The SPA build and the deploy commands are the second consumer of these
parameters, which is why they are published rather than passed through state.

### The API's runtime shape

`crates/server` is an ordinary axum binary and is not modified for Lambda. The
Lambda Web Adapter turns the Lambda invocation into an HTTP request against it:

- Build for `provided.al2023`, name the executable `bootstrap`, and zip it.
- Attach the Lambda Web Adapter as a layer. It provides `/opt/bootstrap`.
- Set `AWS_LAMBDA_EXEC_WRAPPER=/opt/bootstrap`, which makes the adapter the
  entry point and the packaged binary the process it proxies to.
- Set `AWS_LWA_PORT=3000`, matching the address `crates/server` binds.
- Set `AWS_LWA_READINESS_CHECK_PATH=/health`, which the service already serves.

The two `bootstrap` names are unrelated: one is the `provided.al2023` handler
convention for the packaged binary, the other is the adapter's own executable in
the layer.

### Deploying artefacts

Terraform owns the bucket and the function; it does not own their contents
(DR-0005). Artefacts are pushed separately, on their own cadence:

**Frontend.** `trunk build --release`, then sync `dist/` to the bucket and
invalidate CloudFront. Hashed assets are uploaded first and `index.html` last,
so the entry point never references a file that is not there yet.

**API.** Build the `provided.al2023` binary, zip it as `bootstrap`, then
`aws lambda update-function-code`. No Terraform run is involved.

### Configuring the SPA

The SPA needs the API endpoint, the Cognito app client id, and the hosted-UI
domain. They are read from SSM and passed to `trunk build` as environment
variables, which the crate reads at compile time. No hostname is written into
the source — the constraint in `docs/design/frontend.md` holds — and because the
values land inside a content-hashed bundle, a configuration change invalidates
its own cache entry.

## Constraints

- **CloudFront maps both 403 and 404 to `/index.html` with status 200.** Deep
  links are router paths, not files, so without this every reload on a non-root
  route fails — DR-0001. 403 matters as much as 404 here: the origin bucket is
  private behind Origin Access Control, so a missing key returns `AccessDenied`,
  not `NoSuchKey`.

- **The SPA and the API are separate origins, and CORS is configured on the
  HTTP API rather than in `crates/server`.** DR-0001 records the missing CORS
  layer in the service as a gap deployment has to close; deployment closes it at
  API Gateway, whose HTTP APIs answer preflight themselves. That has a second
  benefit: API Gateway answers `OPTIONS` before the JWT authorizer runs, so
  preflight is not rejected for lacking a token. The allowed origin is the
  CloudFront domain. `crates/server` therefore stays free of a CORS layer, and
  local development stays single-origin through the trunk proxy.

  `/api/*` is deliberately not routed through CloudFront as a second origin.
  Doing so would make the SPA single-origin and remove CORS entirely, at the
  cost of putting the API behind a cache the SPA and the API would then have to
  reason about together — the independence DR-0001 chose is worth more.

- **One Cognito app client serves production and local development alike**, so
  its callback and logout URLs list the CloudFront domain *and*
  `http://localhost:8080`, the address in `Trunk.toml`. This is a property of
  the `identity` layer, not a second environment sneaking in.

- **The app client is public and has no client secret.** A secret cannot be kept
  in a WASM bundle the browser downloads; PKCE is what replaces it.

- **`index.html` is served with a short or no-cache header; hashed assets get a
  long one.** trunk emits content-hashed filenames for everything but the entry
  point, so caching `index.html` serves a stale bundle reference to every
  returning visitor.

- **The Lambda's `filename` and `source_code_hash` are under
  `ignore_changes`.** Without it, every `terraform apply` reverts the function to
  whatever placeholder package the configuration names, undoing the last deploy.

- **`crates/server` binds `127.0.0.1:3000` as a constant.** The Lambda
  configuration accommodates it with `AWS_LWA_PORT`; nothing checks that the two
  agree. If the service ever learns to read a port from the environment, that
  variable goes away and this document is what has to change with it.

- **The state bucket carries `prevent_destroy` and versioning.**
  `prevent_destroy` stops Terraform, not the console; versioning is what makes
  the remaining case recoverable.

- **Nothing enforces the apply order between layers.** Terraform sees four
  unrelated root modules. The order in Structure above is maintained by this
  document.

- **Everything lives in one region.** No cross-region resource exists today. A
  custom domain would introduce one, since CloudFront requires its ACM
  certificate in `us-east-1`.
