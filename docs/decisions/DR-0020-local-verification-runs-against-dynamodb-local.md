# DR-0020: Local verification runs against DynamoDB Local, pinned in the development image

Status: accepted
Date: 2026-08-11

## Context

[DR-0018](DR-0018-the-service-runs-without-aws.md) gave `crates/server` two
stores and chose between them from the environment: `TABLE_NAME` set selects
DynamoDB, unset selects a map behind a mutex. Everything a developer runs takes
the second branch, so the first — the key encoding, the `begins_with` query, the
`AttributeValue` mapping, the SDK error path — is compiled by every build and
executed by nothing until it is deployed. DR-0018 named that as the price of its
own decision: two implementations can drift, only one of them is real, and the
tests cover the one that is not.

That price was accepted because the alternative appeared to be an AWS session in
front of every local run. **DR-0018's premise about the remaining alternative was
wrong.** It rejected DynamoDB Local on the grounds that "it needs a container
runtime the development container does not have". DynamoDB Local is a jar; it
needs a JRE. The container image AWS also publishes is one distribution of it,
not the only one. DR-0018's decision still holds — the in-memory store remains
the default, for the reason it gives — but the alternative it dismissed turns out
to be available, so its "one implementation instead of two" benefit can be had
for verification without giving up the no-setup default.

The occasion for looking again was packaging the API artefact. Changing how the
deployed binary is built is not something to attempt while the deployed store has
never been run outside AWS.

## Decision

DynamoDB Local is pinned into the devcontainer image and is what local
verification runs the DynamoDB half of the service against.

`.devcontainer/Dockerfile` installs `default-jre-headless` and unpacks a
date-pinned archive, checked against AWS's published SHA-256, into
`/opt/dynamodb-local`. `just` recipes are the interface: `dynamo` runs it in
memory on port 8000 with `-sharedDb` and `-disableTelemetry`, `dynamo-table`
creates the table idempotently through the `aws` CLI already in the image, and
`dev-api-dynamo` runs the ordinary `cargo run -p server` with `TABLE_NAME`,
`AWS_ENDPOINT_URL_DYNAMODB`, a region and deliberately fake credentials.
`dynamo-stop` is beside them for when Ctrl-C is not available; it reads `/proc`,
because the image has no `ps`, `pkill` or `lsof`.

Nothing here reports to AWS. A verification step whose point is that it needs no
AWS account should not be telling one that it ran, which is what
`-disableTelemetry` is for.

`crates/server` is not changed, and this decision is conditional on that
remaining true. `AWS_ENDPOINT_URL_DYNAMODB` is read by the SDK's generated
config rather than by the service, so the store is selected by `TABLE_NAME`
exactly as it is on the Lambda. What runs locally is the deployed code path,
configured differently — not a second path that resembles it.

The in-memory store stays, stays the default, and is still what `just dev-api`
runs. This is an opt-in verification mode beside it, not a replacement: a fresh
clone still starts with no credentials and no setup, which is DR-0008's principle
and the whole of DR-0018's point.

## Alternatives

- **Leave the DynamoDB half unrun until deployment.** The status quo DR-0018
  accepted. Rejected now because the next piece of work changes how the artefact
  is built, and a failure would then have two candidate causes — the packaging or
  the store — with no way to separate them.
- **Point development at the real table.** Rejected for DR-0018's reasons, which
  are undisturbed: an AWS session in front of every local run, and development
  rows in a production table. It remains available, and is now the thing the fake
  credentials in `dev-api-dynamo` exist to make impossible by accident.
- **DynamoDB Local as a container.** The devcontainer has no Docker, and giving
  it one to run a jar is a large dependency for a small need.
- **A hand-written DynamoDB fake in Rust.** Rejected outright: a third
  implementation of the same behaviour, drifting from the other two, and the
  thing being verified is precisely the SDK's own encoding of the request.
- **Install the JRE per developer rather than in the image.** Rejected because
  the image is the only place the project states its tools, and a verification
  step that works on one machine and not another is not a verification step.

## Consequences

The DynamoDB path can be exercised before it is deployed. Owner isolation, the
`TYPE#` prefix, the fixed-width `created_at` and the query behind the action
types list are observable against a real DynamoDB implementation, with `scan`
available locally to inspect what was actually stored — a query the deployed
function is deliberately not granted.

The development image carries a JVM, around 200 MB, for one tool. Nothing in the
workspace compiles to it and no Java is written. This is affordable only because
the artefact is built in its own image and no longer inherits the development
environment; a JRE here cannot reach what is deployed.

**Without `-sharedDb`, DynamoDB Local partitions its databases by access key and
region.** The CLI creating the table and the server querying it would then
address two different databases, both would succeed, and nothing would report it.
The flag is in the `dynamo` recipe and the reason is in the comment above it.

`-inMemory` means the table does not survive a restart, so `dynamo-table` is
re-run each time — the same lifetime the in-memory store has, which keeps the two
modes behaving alike in the one respect a developer notices.

Two more values are now maintained by hand against `infra/`: the region and the
table name in the `justfile`, joining `project`, which was already kept in step
that way.

The drift DR-0018 accepted is not eliminated. The in-memory store is still a
second implementation, `cargo test` still covers only it, and nothing runs
`dev-api-dynamo` automatically. What changes is that checking the other one is
now three commands rather than a deployment.
