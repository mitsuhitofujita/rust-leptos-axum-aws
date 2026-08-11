# Backend

Updated: 2026-08-11

## Purpose

The axum service in `crates/server`: what it exposes, who it thinks is calling,
and where it keeps what it is given.

It is an ordinary axum binary. Nothing in it is written for Lambda — the Lambda
Web Adapter turns an invocation into an HTTP request on `127.0.0.1:3000`, and
`just dev-api` runs the same binary with nothing in front of it (DR-0001).

## Structure

| File | Role |
| --- | --- |
| `src/main.rs` | The router, the shared state, and which store the process is using |
| `src/identity.rs` | Who the caller is |
| `src/store.rs` | Reading and writing the table |
| `src/action_types.rs` | `/api/action-types`, and what may be stored |
| `src/dashboard.rs` | `/api/dashboard`, still answering from fixed values |

`Arc<Store>` is the router's state, built once at startup. The SDK client is
expensive to construct and cheap to share, and which store the process is using
cannot change while it runs.

**Identity.** `identity::Owner` is an axum extractor, and it is the whole of
user isolation. A handler asks for the owner and cannot ask for anything else:
no path, query or body parameter names one, which is what stops a handler from
serving a partition its caller does not own. The value is the Cognito `sub`,
read out of the `x-amzn-request-context` header the Lambda Web Adapter forwards
the API Gateway request context in (DR-0017).

The service never validates a token, holds no JWT library, and has no public
keys. API Gateway's JWT authorizer has already run and is the only enforcement
point there is (DR-0010).

A request without that header is attributed to a constant development owner,
because there is no adapter and no authorizer in front of `just dev-api`
(DR-0018).

**The store.** `store::Store` is an enum with two variants, chosen from the
environment at startup: `TABLE_NAME` set selects DynamoDB, unset selects an
in-memory map. Terraform sets it on the Lambda and nothing sets it locally, so
the deployed service and the development server differ by configuration rather
than by code (DR-0018). An enum rather than a trait object because there are
exactly two and the choice is settled before the first request.

The DynamoDB variant is run locally by `just dev-api-dynamo`, against the
DynamoDB Local pinned in the devcontainer image (DR-0020). That recipe sets
`TABLE_NAME` and points the SDK at `http://localhost:8000` with fake
credentials; the binary is the same one, taking the same branch it takes on the
Lambda, which is what makes it a check of the deployed path rather than of
something resembling it.

Keys, attributes and queries are `persistence.md`'s, not this document's. What
belongs here is that the identifier is minted by the service — a ULID, so it
sorts by creation time and needs no coordination — and that what leaves the
service is the bare ULID, never the `TYPE#`-prefixed key.

**Validation.** `action_types::validate` is where a request stops being
whatever was sent. Names and units are trimmed and then required to be
non-empty, so a field of spaces is refused where the browser's `required`
attribute would accept it; both are length-limited, counted in characters. The
icon must be a name in `shared::icon_names`, the same catalog the picker was
generated from — the picker is the only control surface that offers one, but a
request need not have come from it (DR-0014, DR-0019).

**Failures.** `action_types::Failure` separates the two kinds. A rejected
request answers `400` with the reason in plain words, because that reason is
what the screen shows; a store that did not answer is logged and answered `500`
with a sentence the visitor can do nothing with, because there is nothing they
could do.

## Interfaces

**Exposes**

| Route | Answers |
| --- | --- |
| `GET /health` | `ok`. Routed outside the authorizer, because a probe carries no token |
| `GET /api/dashboard` | `shared::Dashboard`, from hardcoded values |
| `GET /api/action-types` | `shared::ActionType[]`, oldest first |
| `POST /api/action-types` | `201` and the stored `shared::ActionType`, from a `shared::NewActionType` |

No CORS layer. Development is single-origin through the trunk proxy, and
production is answered by the HTTP API rather than here (DR-0009). A new method
under `/api` needs `local.api_methods` in `infra/api/apigateway.tf` to name it;
a new path does not.

**Depends on** `axum` and `tokio`, `aws-config` and `aws-sdk-dynamodb`,
`ulid`, `time` for one formatted instant, `serde` and `serde_json` for the
request context, and `shared`.

**Reads** `TABLE_NAME` from the environment, and nothing else. The SDK reads its
own variables underneath — the region, the credentials, and the
`AWS_ENDPOINT_URL_DYNAMODB` that `just dev-api-dynamo` redirects the client with.
None of them is named in this crate, which is why running against a local
DynamoDB cost it no code (DR-0020).

## Constraints

- **The owner comes from the request context and from nowhere else.** The IAM
  policy cannot express user isolation — the function serves every user, so its
  permissions cover every partition — so a handler that took an owner from a
  request parameter would defeat it entirely — DR-0010, DR-0017.
- **The `x-amzn-request-context` header is not a security boundary.** Anyone who
  could reach the service directly could forge it; nothing can, because API
  Gateway is the only route to the function and overwrites it on every request.
  Exposing the service by any other path invalidates this — DR-0017.
- **A missing header means development, not rejection.** In a deployed function
  it does not occur. If it ever did, the write would land in the development
  owner's partition rather than failing — DR-0018.
- **`created_at` and the instant inside a record's sort key must be fixed-width
  RFC 3339 in UTC.** `store::TIMESTAMP` is the only thing enforcing it, and a
  variable-width instant fails silently — DR-0015.
- **The in-memory store is not a second design, and can still drift.** It
  answers from insertion order where DynamoDB answers a `Query` in key order,
  and those agree only because the key embeds a ULID. Anything that changes the
  key encoding changes both — DR-0018. `cargo test` reaches only the in-memory
  half; the other one is checked by running `just dev-api-dynamo` by hand, which
  nothing does automatically — DR-0020.
- **`Scan` is not granted and no access pattern needs one.** Every query is
  inside one owner's partition — `persistence.md`.
- **The dashboard is not connected to the store.** It answers from values in
  `src/dashboard.rs`. Only the body of that handler changes when it is.
