# A local DynamoDB the service can be verified against

Status: in progress
Started: 2026-08-11
Branch: main

## Request

Before the work in `docs/work/2026-08-10-api-artefact-packaging.md` is attempted,
establish a way to verify the system locally. The two parts of the deployed edge
that `crates/server` is written against but never exercises outside AWS are the
DynamoDB table and the authentication filter — API Gateway's JWT authorizer
together with the request context the Lambda Web Adapter forwards. Both are to be
reproduced on a developer's machine, following whatever is the best practice for
doing so.

The result is meant to be used for a long time, so it is to be built to last
rather than as a throwaway. Anything that has to be written is written in Rust:
this is a devcontainer and Python is not available. `ripgrep` is.

The four-phase shape proposed in conversation is accepted and all four phases are
to be carried out. The work may be split into Work Logs as seems best, ordered so
that each piece can be carried out on its own and the sequence can be abandoned
part way through, smallest first.

**This log answers the first phase: the local store.** The others are
`2026-08-11-local-api-edge.md`, `2026-08-11-local-token-verification.md` and
`2026-08-11-end-to-end-verification.md`. Nothing here depends on any of them.

## Interpretation

**What is being asked.** Today `just dev-api` runs the in-memory half of
`store::Store` (DR-0018). The DynamoDB half — the key encoding, the
`begins_with` query, the `AttributeValue` mapping, the SDK error path — is
compiled but never run until it is deployed. DR-0018 named that drift as the
price of its own decision. This phase makes the deployed half runnable locally,
against the real DynamoDB implementation rather than a stand-in for it.

**The shape of the answer.** DynamoDB Local, AWS's own build, reached through
`AWS_ENDPOINT_URL_DYNAMODB`. That environment variable is read by the generated
SDK config, so selecting it costs `crates/server` no code at all: setting
`TABLE_NAME` is still what chooses `Store::Dynamo`, exactly as in the deployed
function. Verified before this log was opened — see Progress.

**Out of scope.**

- `crates/server`. If this phase needs a line of it, the approach is wrong.
- The authorizer, the request context header, CORS and the route table. Those are
  the second phase's, and this phase deliberately does not need them: the header
  can be passed by hand with `curl` while verifying.
- The in-memory store, which stays and stays the default. DR-0008's principle —
  an unset variable means something workable — is what makes a fresh clone run
  with no setup, and DynamoDB Local is an opt-in verification mode rather than a
  replacement for it.
- Anything about how the artefact is packaged. That is the log this one precedes.

**Assumptions.**

- The devcontainer image may gain a development tool. The packaging decision
  already taken — the artefact is built inside its own image and stops inheriting
  the development environment — is precisely what makes this free: a JRE in the
  devcontainer cannot reach the artefact.
- A JRE is acceptable as the runtime for an AWS-published jar. It is present for
  no other purpose, nothing in the workspace compiles to it, and no Java is
  written. If it is not acceptable, this phase has no good answer and the
  alternatives are recorded below.
- The developer running this has no AWS credentials and needs none.

## Plan

1. **Pin DynamoDB Local into the devcontainer image.** Add a headless JRE to the
   existing `apt-get install` list in `.devcontainer/Dockerfile`, and unpack the
   dated archive — not `dynamodb_local_latest.zip` — into `/opt/dynamodb-local`,
   pinned by an `ARG` in the same style as `TERRAFORM_VERSION` and
   `TRUNK_VERSION`. Comment the JRE's role at the point it is installed, so that
   nobody has to guess why Java is in a Rust image.
2. **`just dynamo`** — run it in memory, on port 8000, with `-sharedDb`.
3. **`just dynamo-table`** — create the table, idempotently. The key schema is a
   copy of `infra/data/main.tf`'s; the recipe comment cites
   `docs/design/persistence.md` as the interface it is a copy of. The `aws` CLI
   is already in the image and already how every other recipe reaches AWS, so
   this adds no tooling.
4. **`just dev-api-dynamo`** — `cargo run -p server` with `TABLE_NAME`,
   `AWS_ENDPOINT_URL_DYNAMODB`, a region and deliberately fake credentials.
5. **Write the two pitfalls into the recipes themselves**, where they will be
   read: that without `-sharedDb` DynamoDB Local partitions tables by access key
   and region, so the CLI and the service would silently see different tables;
   and that the fake credentials are a feature, because they make it impossible
   for a local run to reach the real table by accident.
6. **Documents.** A Decision Record for the store used in local verification,
   which also corrects DR-0018's premise (below). Draft updates to
   `workspace.md` (the image's contents, the new recipes) and `backend.md` (how
   the DynamoDB half is exercised), for confirmation.

## Progress

### 2026-08-11 — the premise was verified before the plan was written

Every claim the plan rests on was checked by hand first, in a scratchpad outside
the repository, because the whole approach fails if any one of them is false.

- **DynamoDB Local needs a JRE and nothing else.** Version 3.3.1 started on
  Temurin JRE 21 and answered the CLI. It is distributed as a zip, so `unzip` —
  already in the image — is enough and no `xz` is needed.
- **The archive can be pinned.** `dynamodb_local_2026-07-31.zip` exists beside
  `dynamodb_local_latest.zip` at the CloudFront distribution and a fabricated
  date 404s, so the dated path is real rather than an alias.
- **`crates/server` needs no change.** `aws-sdk-dynamodb` 1.120.0 builds the
  service-specific key at `src/config.rs:1526`
  (`service_config_key("DynamoDB", "AWS_ENDPOINT_URL", "endpoint_url")`), so
  `AWS_ENDPOINT_URL_DYNAMODB` redirects the client. Started with `TABLE_NAME`
  set, the unmodified binary printed `action types are stored in DynamoDB table
  rust-leptos-axum-aws-app` and served a `POST` and a `GET` against it.
- **The key encoding is what `persistence.md` says it is.** The item the service
  wrote was `pk=USER#local-user-1`, `sk=TYPE#01KZQB4Y73S2CZEV3AGHP9PE4H`,
  `created_at=2026-08-11T02:41:41.603Z` — 24 characters, fixed width.
- **Owner isolation is real.** The same `GET` with no `x-amzn-request-context`
  header answered `[]`, because it was reading the development owner's partition
  rather than `local-user-1`'s.

None of this is committed; the scratchpad copies of the JRE and the jar are
outside the repository and are not what the plan installs.

### 2026-08-11 — a correction DR-0018 needs

DR-0018 rejected DynamoDB Local on the grounds that "it needs a container runtime
the development container does not have". That premise is wrong: DynamoDB Local
is a jar, and needs a JRE. The container image is one distribution of it, not the
only one.

The decision itself still holds — the in-memory store remains the default, and
for the reason DR-0018 gives. What changes is that the alternative it dismissed
turns out to be available, so its "one implementation instead of two" benefit can
be had for verification without giving up the no-setup default. Decision Records
are append-only, so DR-0018 is not edited; the new record carries the correction
in its Context.

## Verification

To be recorded. The intended check is the one already run by hand above, made
repeatable: `just dynamo`, `just dynamo-table`, `just dev-api-dynamo`, then a
`POST` and a `GET` carrying a hand-made request context header, and a `scan`
confirming the stored key encoding.

## Retirement

- [ ] Design Documents updated — `workspace.md`, `backend.md`
- [ ] Decision Records written (DR-____) — DynamoDB Local as the store local
      verification runs against, and the correction to DR-0018's premise
- [ ] Non-obvious knowledge preserved — the `-sharedDb` partitioning trap; that
      fake credentials are what stop a local run reaching the real table; that
      `AWS_ENDPOINT_URL_DYNAMODB` is what keeps `crates/server` untouched
- [ ] No durable document depends on this log
