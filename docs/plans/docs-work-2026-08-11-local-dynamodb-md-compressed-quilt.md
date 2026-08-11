# Phase 1: a local DynamoDB the service can be verified against

Executes `docs/work/2026-08-11-local-dynamodb.md`.

## Context

`crates/server` picks its store from the environment: `TABLE_NAME` set selects
DynamoDB, unset selects an in-memory map (DR-0018). Everything a developer runs
takes the second branch, so the first — the key encoding, the `begins_with`
query, the `AttributeValue` mapping, the SDK error path — is compiled by every
build and executed by nothing until it is deployed. DR-0018 named that drift as
the price of its own decision and rejected DynamoDB Local because "it needs a
container runtime the development container does not have". That premise is
wrong: DynamoDB Local is a jar and needs a JRE.

This phase makes the deployed half runnable on a developer's machine, against
the real DynamoDB implementation rather than a stand-in, without touching a line
of `crates/server`. It is the first of the four phases the Work Log describes and
depends on none of the others. The in-memory store stays, and stays the default.

Confirmed with the user before planning: the JRE in the devcontainer image is
accepted, and the image change is made first and verified after the user rebuilds
the container.

## Facts established while planning

- `https://d1ni2b6xgvw0s0.cloudfront.net/v2.x/dynamodb_local_2026-07-31.zip`
  exists (50,408,163 bytes) and AWS publishes `…zip.sha256` beside it:
  `5b0d17dd3b4e929db64a9f624a3f96eaf0961e3cf4acece00091656aec5fc7ed`. The
  `latest` archive has the same digest today, so the dated path is the same build
  and is what gets pinned. There is no `v3.x` path — the segment is the
  distribution's, not the build's version.
- Debian trixie's `default-jre-headless` is `openjdk-21-jre-headless`; the image
  has no `java` today.
- The single region in `infra/*/variables.tf` is `ap-northeast-1`; the table is
  `"${var.project}-app"` in `infra/data/main.tf`, i.e. `rust-leptos-axum-aws-app`.
- `aws-cli/2.36.16` is in the image and honours `AWS_ENDPOINT_URL_DYNAMODB`.

## Changes

### 1. `.devcontainer/Dockerfile`

Both edits go in the root-owned section, above the `VSCODE_UID` block at line 32.

- Add `default-jre-headless` to the existing alphabetical `apt-get install` list
  (line 6-13), with a comment line inside the continuation stating why a Rust
  image has a JVM: DynamoDB Local is a jar, nothing in the workspace compiles to
  the JVM, and no Java is written. (Whole-line comments inside a continued
  instruction are stripped by the Dockerfile parser. If the build rejects it,
  move the comment above the `RUN`.)
- Below the `TERRAFORM_VERSION` block (line 22-30), in the same style, pin and
  unpack the archive:

  ```dockerfile
  ARG DYNAMODB_LOCAL_VERSION=2026-07-31
  ARG DYNAMODB_LOCAL_SHA256=5b0d17dd3b4e929db64a9f624a3f96eaf0961e3cf4acece00091656aec5fc7ed
  RUN curl -fsSL -o /tmp/dynamodb-local.zip \
          "https://d1ni2b6xgvw0s0.cloudfront.net/v2.x/dynamodb_local_${DYNAMODB_LOCAL_VERSION}.zip" && \
      echo "${DYNAMODB_LOCAL_SHA256}  /tmp/dynamodb-local.zip" | sha256sum -c - && \
      unzip -q /tmp/dynamodb-local.zip -d /opt/dynamodb-local && \
      rm /tmp/dynamodb-local.zip
  ```

  The comment above it says what it is for, that the dated archive is used rather
  than `dynamodb_local_latest.zip` so the image is reproducible, that the digest
  is AWS's published one and is what makes the pin real, and that `v2.x` is the
  distribution path rather than the version. `unzip` is already installed for
  Terraform's sake, so this adds no tooling.

Note for the reviewer: this is a ~200 MB layer, and it exists only in the
development image. The artefact is built in its own image (the packaging work
this phase precedes), so a JRE here cannot reach what is deployed.

### 2. `justfile`

A new section between `dev-api` (line 38-39) and `build`, with three top-level
assignments and three recipes. The section comment states the point: the DynamoDB
half runs here instead of only in AWS, and `crates/server` is untouched because
`AWS_ENDPOINT_URL_DYNAMODB` is read by the SDK's generated config while
`TABLE_NAME` still chooses the store, exactly as on the Lambda.

```just
dynamo_endpoint := "http://localhost:8000"
dynamo_region := "ap-northeast-1"
dynamo_table := project + "-app"
```

`dynamo_region` mirrors `variable "region"`, and `dynamo_table` mirrors
`"${var.project}-app"`; both are kept in step with `infra/` by hand, like
`project` at the top of the file already is.

- **`dynamo`** — foreground, like `dev-api`:

  ```just
  java -Djava.library.path=/opt/dynamodb-local/DynamoDBLocal_lib \
       -jar /opt/dynamodb-local/DynamoDBLocal.jar \
       -inMemory -sharedDb -port 8000
  ```

  Its comment carries the first pitfall: **`-sharedDb` is not optional.** Without
  it DynamoDB Local keeps a separate database per access key and region, so
  `dynamo-table` and the server would address two different tables and neither
  would ever say so. `-inMemory` also means the table is gone when this stops —
  the same lifetime the in-memory store has, so `dynamo-table` is re-run after
  every restart.

- **`dynamo-table`** — a bash recipe that exports the endpoint, the region and
  the fake credentials, returns early if `describe-table` succeeds, and otherwise
  `create-table`s with `pk` HASH / `sk` RANGE, both `S`, `--billing-mode
  PAY_PER_REQUEST`. Its comment says the key schema is a copy of
  `infra/data/main.tf`'s and cites `docs/design/persistence.md` as the interface
  both copies follow.

- **`dev-api-dynamo`** — `cargo run -p server` with `TABLE_NAME`,
  `AWS_ENDPOINT_URL_DYNAMODB`, `AWS_REGION` and the fake credentials.

  Its comment carries the second pitfall: **the fake credentials are a feature.**
  DynamoDB Local checks nothing, and a process that cannot authenticate anywhere
  cannot reach the real table by accident — including when a real AWS session is
  configured, which these override.

No `dynamo-scan` recipe. `Scan` is deliberately not granted in production and a
scan against this table is a defect (`persistence.md`); the one-off scan used to
inspect the key encoding belongs in the Work Log's Verification section, not in
the task runner.

### 3. Documents

- **`docs/decisions/DR-0020-…md`** (new, next free number). Decision: local
  verification runs against DynamoDB Local, pinned in the devcontainer image and
  reached through `AWS_ENDPOINT_URL_DYNAMODB`; the in-memory store remains the
  default. Context carries the correction to DR-0018's premise — DR-0018 is
  append-only and is **not** edited, and its Status stays `accepted`, because the
  decision it records still holds. Alternatives: the real table (DR-0018's
  reasons stand), a container-based DynamoDB Local (no Docker), a hand-written
  fake in Rust (a third implementation to drift), and leaving the DynamoDB half
  unrun. Consequences: a JVM in the development image; the `-sharedDb`
  partitioning trap; data lost on restart; two terminals instead of one; and the
  drift DR-0018 accepted is now checkable rather than merely acknowledged.
- **`docs/design/workspace.md`** (draft, human confirms). The image gains a
  headless JRE and a pinned DynamoDB Local under `/opt/dynamodb-local`; three
  rows in the recipe table; and one line keeping the current claim honest —
  `dev-api` and `dev-web` still need no credentials and no setup, and this is an
  opt-in verification mode beside them.
- **`docs/design/backend.md`** (draft, human confirms). Under **The store**, how
  the DynamoDB half is exercised locally. The constraint "The in-memory store is
  not a second design, and can still drift" gains the way to check it. **Reads**
  gains a clause: `TABLE_NAME` is still the only variable the service reads, and
  the endpoint variable is the SDK's, not the service's — which is why this cost
  no code.
- **`docs/design/persistence.md`** (one sentence, human confirms). "The table" is
  described as created by the `data` layer; add that a local copy of the same key
  schema is created by `just dynamo-table` for verification, citing DR-0020.
- **`docs/work/2026-08-11-local-dynamodb.md`**. Progress entries as the work is
  done, the Verification section filled in with what was actually run, and the
  Retirement checklist completed. The log is deleted only once a human has
  confirmed the Design Document updates — that is a separate step, not part of
  this one.

## Verification

Before the rebuild, from here:

1. `just --list` and `just --evaluate` parse the new recipes and assignments.
2. `rg -n "TABLE_NAME|AWS_ENDPOINT" crates/server` shows no change — no Rust file
   is touched by this phase at all.

After the user rebuilds the devcontainer:

3. `java -version` and `ls /opt/dynamodb-local` — the JRE and `DynamoDBLocal.jar`
   are present.
4. Terminal A: `just dynamo` — listening on 8000.
5. `just dynamo-table`, then `just dynamo-table` again — created, then reported as
   already existing, with no error.
6. Terminal B: `just dev-api-dynamo` — the startup line reads `action types are
   stored in DynamoDB table rust-leptos-axum-aws-app`.
7. With `-H 'x-amzn-request-context: {"authorizer":{"jwt":{"claims":{"sub":"local-user-1"}}}}'`:
   `POST /api/action-types` answers `201`, and `GET /api/action-types` returns
   what was written.
8. The same `GET` **without** the header answers `[]` — it reads the development
   owner's partition, which proves owner isolation is real against the table
   rather than against the map.
9. `aws dynamodb scan` against the local endpoint shows `pk=USER#local-user-1`,
   `sk=TYPE#<26-character ULID>` and a fixed-width `created_at`, matching
   `docs/design/persistence.md`.
10. `just dev-api` with nothing set still starts on the in-memory store, so the
    no-setup default is untouched.
