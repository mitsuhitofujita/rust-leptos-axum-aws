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

### 2026-08-11 — the plan carried out, except for what needs a rebuilt container

Confirmed with the requester before writing anything: the JRE in the image is
accepted, and the image change is made first and verified after the container is
rebuilt rather than rehearsed a second time in a scratchpad.

- **`.devcontainer/Dockerfile`.** `default-jre-headless` joins the apt list, with
  a comment inside the continuation saying why a Rust image has a JVM. Below the
  `TERRAFORM_VERSION` block, `DYNAMODB_LOCAL_VERSION=2026-07-31` and its SHA-256
  unpack the archive into `/opt/dynamodb-local`. Both are in the root-owned half
  of the file, above the `USER vscode` switch.
- **The digest is AWS's own.** `dynamodb_local_2026-07-31.zip.sha256` is
  published beside the archive and reads
  `5b0d17dd…c7ed`, the same digest `dynamodb_local_latest.zip.sha256` carries
  today — so the dated URL is the same build, and the checksum is what turns a
  dated name into a pin. Only `v2.x` exists as a path segment; `v3.x` 404s, so
  the segment is the distribution's and not the build's version.
- **`justfile`.** A `Local verification against DynamoDB` section between
  `dev-api` and `build`: three assignments (`dynamo_endpoint`, `dynamo_region`,
  `dynamo_table`, the last two mirroring `infra/`), and `dynamo`, `dynamo-table`,
  `dev-api-dynamo`. Both pitfalls are in the comments above the recipes they
  belong to. `just --list` and `just --evaluate` parse them.
- **Each recipe's explanation goes above a blank line and a one-line summary**,
  the shape `dev-web-auth` already uses, because `just --list` shows only the
  last comment line and a paragraph ending mid-sentence is what the existing
  `icons` and `tf-validate` entries look like.
- **No `dynamo-scan` recipe.** `Scan` is deliberately not granted in production
  and a scan of this table is a defect (`persistence.md`); the one used to
  inspect the key encoding belongs in Verification below, not in the task runner.
- **`crates/server` is untouched**, as the Interpretation required. `git status`
  lists `.devcontainer/Dockerfile`, `justfile` and documents, and nothing else.
- **Documents.** DR-0020 written, and drafts made to `workspace.md`,
  `backend.md`, `persistence.md` and the index. `persistence.md` was not in the
  plan's list; it describes the table as the `data` layer's alone, which is no
  longer the whole truth now that a second copy of the key schema exists, so it
  gained a sentence pointing at `just dynamo-table`.
- **The download host was queried on review.** `d1ni2b6xgvw0s0.cloudfront.net`
  reads like this project's own distribution and is not: it is AWS's public
  download endpoint for DynamoDB Local, serving the archive and its checksum to
  anyone unauthenticated behind the default `*.cloudfront.net` certificate. This
  project's distribution domain is written down nowhere — it is an output
  published to SSM and resolved at deploy time (DR-0005). The added lines were
  audited for `arn:`, twelve-digit ids, access-key prefixes and account names;
  the only matches are that URL and the deliberately fake `local` credentials.
  The Dockerfile comment now says so, because the same alarm will otherwise be
  raised by the next reader.
- **DR-0018 is not edited.** Its Status stays `accepted`: the decision holds and
  only its account of a rejected alternative was wrong, and that correction lives
  in DR-0020's Context, where an append-only record can carry it.

### 2026-08-11 — verified against the rebuilt container

The container was rebuilt and every unticked check below the rule was run. All of
them pass, unchanged from what the plan predicted, with one thing worth writing
down: `POST /api/action-types` takes `name`, `unit` and `icon`, and a body
carrying only `name` is rejected with `missing field 'unit'` before the store is
reached. The Verification section said only "`POST … answers 201`" and so did not
say what to post; it now carries the body that works.

Nothing needed fixing. The image, the recipes and the documents are as they were
written before the rebuild.

### 2026-08-11 — `dynamo-stop`, asked for after the verification

`just dynamo` runs in the foreground, so Ctrl-C is the ordinary way to stop it;
a recipe was asked for besides, for when the terminal that started it is gone or
port 8000 is held by something unobvious.

The image has nothing to find a process with. `ps`, `pgrep`, `fuser`, `lsof`,
`ss` and `netstat` are all absent — procps is not installed — and `pkill` is
worse than absent: a shell may define it as a function, so `command -v pkill`
answers and running it fails with `command not found`. DynamoDB Local has no
shutdown endpoint either; `POST /shutdown` and `GET /shutdown` both answer `400`
on 3.3.1, and `-help` lists no shutdown option, so a signal is the only way.

`dynamo-stop` therefore reads `/proc` directly, in bash like `dynamo-table`,
matching **both** `comm` = `java` and the exact jar path among the NUL-separated
arguments. The path alone is not enough: the scanning shell's own command line
contains it, and a first attempt killed itself — the `comm` test is what excludes
it. Nothing was added to the image, so no rebuild is needed for this.

A Rust helper was considered, as CLAUDE.md prefers for tooling, and rejected: it
would be a crate and a build step in front of a `kill`, where the loop is eight
lines and the recipe beside it is already bash.

Killing the JVM is enough — `just`'s own `sh -cu` wrapper exits with it, observed
as exit 143 for the whole `just dynamo`.

- [x] The `justfile` parses, the three recipes appear in `just --list` with
      readable summaries, and `dynamo_table` evaluates to
      `rust-leptos-axum-aws-app`.
- [x] No file under `crates/` is modified.
- [x] The `aws` CLI honours `AWS_ENDPOINT_URL_DYNAMODB`: with nothing listening,
      `describe-table` fails with `Could not connect to the endpoint URL:
      "http://localhost:8000/"` rather than reaching AWS. It follows that
      `dynamo-table` run without `dynamo` fails loudly rather than creating
      anything anywhere.

---

After `Dev Containers: Rebuild Container`, in two terminals:

- [x] `java -version` answers — `openjdk 21.0.12`, Debian's
      `default-jre-headless` — and `/opt/dynamodb-local/DynamoDBLocal.jar` exists
      beside `DynamoDBLocal_lib`.
- [x] `just dynamo` listens on 8000, announcing `Version: 3.3.1`, `InMemory:
      true`, `SharedDb: true` — the flags the recipe passes, echoed back.
- [x] `just dynamo-table` prints `rust-leptos-axum-aws-app` and exits 0; run
      again it prints `rust-leptos-axum-aws-app already exists` and exits 0.
- [x] `just dev-api-dynamo` prints `action types are stored in DynamoDB table
      rust-leptos-axum-aws-app`.
- [x] With `-H 'x-amzn-request-context:
      {"authorizer":{"jwt":{"claims":{"sub":"local-user-1"}}}}'` and the body
      `{"name":"Running","unit":"km","icon":"footprints"}`, `POST
      /api/action-types` answers `201` with
      `{"id":"01KZQSTT08MG7D8G0F3G51V06C",…}`, and `GET /api/action-types`
      returns that one item. All three fields are required: a body with only
      `name` is rejected with `missing field 'unit'`.
- [x] The same `GET` without the header answers `[]` — the development owner's
      partition, which is owner isolation observed against the table rather than
      against the map.
- [x] `aws dynamodb scan` against the local endpoint shows one item:
      `pk=USER#local-user-1`, `sk=TYPE#01KZQSTT08MG7D8G0F3G51V06C` — 26
      characters — and `created_at=2026-08-11T06:58:18.248Z`, 24 characters,
      matching `docs/design/persistence.md`.
- [x] `just dev-api` with nothing set still prints `action types are stored in
      memory (no TABLE_NAME is set)`.

`dynamo-stop`, added afterwards:

- [x] With DynamoDB Local running, `just dynamo-stop` prints `stopped DynamoDB
      Local (pid …)`, exits 0, and port 8000 is free a second later.
- [x] Run with nothing running it prints `DynamoDB Local is not running` and
      exits 0, so it is as idempotent as `dynamo-table`.
- [x] It stops `just dynamo` and not merely the JVM: the whole recipe ends,
      exit 143.
- [x] A full round trip — `dynamo`, `dynamo-table`, `dynamo-stop` — leaves the
      port free, and `just --evaluate` and `just --list` still parse, the latter
      showing the recipe between `dynamo` and `dynamo-table`.

## Retirement

- [ ] Design Documents updated — `workspace.md`, `backend.md`, and
      `persistence.md` and `index.md` besides. Drafted; awaiting confirmation,
      which `docs/README.md` requires before an overwrite counts as done.
- [x] Decision Records written — DR-0020, which records DynamoDB Local as the
      store local verification runs against and carries the correction to
      DR-0018's premise in its Context. DR-0018 itself is unedited.
- [x] Non-obvious knowledge preserved — the `-sharedDb` partitioning trap (DR-0020
      Consequences, `workspace.md` Constraints, and the recipe comment); that fake
      credentials are what stop a local run reaching the real table (DR-0020
      Alternatives and the recipe comment); that `AWS_ENDPOINT_URL_DYNAMODB` is
      the SDK's variable and not the service's, which is what keeps
      `crates/server` untouched (DR-0020 Decision, `backend.md` Interfaces); and
      that the image has no process tools at all, with `pkill` possibly defined
      as a shell function over nothing (`workspace.md` Constraints, and the
      `dynamo-stop` recipe comment).
- [x] No durable document depends on this log
- [x] Verification completed after the container rebuild — every check in the
      section above is ticked, run against the rebuilt image on 2026-08-11.
