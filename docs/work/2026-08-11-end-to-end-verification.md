# End-to-end verification against the local rig

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

**This log answers the fourth phase: automated end-to-end tests over the rig.**
The others are `2026-08-11-local-dynamodb.md`, `2026-08-11-local-api-edge.md` and
`2026-08-11-local-token-verification.md`. This phase needs the first two and does
not need the third.

## Interpretation

**What is being asked.** The first three phases produce something a person can
run. This one makes it something that runs itself, so that the properties the
rig can now observe are asserted rather than remembered. Without it the rig
decays into three commands nobody types.

**What the tests are for.** One list, and it is not a general test suite for the
application. Every case is a property that is currently unobservable outside AWS,
and most are drawn from the consequences the durable layer already admits to:

| Property | Why it is here |
| --- | --- |
| Action types come back in creation order from a real `Query` | DR-0018 says the two stores agree only because the key embeds a ULID; nothing checks it |
| The key encoding is `USER#…` / `TYPE#<ulid>` and `created_at` is 24 characters | DR-0015; `store::TIMESTAMP` is the only thing enforcing it |
| One owner cannot see another's types | The single reason `identity::Owner` exists |
| A request with no token is refused before the service is reached | DR-0010 |
| A method not in `local.api_methods` is a 404 | DR-0009's trap |
| A preflight is answered without a token | DR-0009 |
| A non-string claim degrades to the development owner | The silent failure the second phase names |
| A forged `x-amzn-request-context` does not reach the service | DR-0017's argument, made visible |

**Out of scope.**

- The SPA. There is no browser and no Node.js in this container, and
  `workspace.md` already records that as a boundary. These tests drive HTTP.
- Anything requiring AWS. The tests run with no credentials and no network.
- Performance, cold starts, throttling, IAM. The rig cannot observe any of them
  and the tests must not pretend to.
- Making these part of `cargo test --workspace`. They need a JRE and built
  binaries, so a plain `cargo test` on a fresh clone must not start failing.

**Assumptions.**

- Spawning processes from a test is acceptable here. The alternative — importing
  the service as a library — would mean restructuring `crates/server` around
  testability, which is the coupling the whole approach avoids.
- Fixed non-default ports are enough to avoid colliding with a running
  development session; the tests do not need to be runnable concurrently with
  themselves.

## Plan

1. **`crates/e2e`**, a development-only crate whose tests spawn the three
   processes: DynamoDB Local, `server`, and `devgateway` in its `local` mode.
   Ports well away from 8000/3000/3001 so a development session can keep running.
2. **The harness** creates the table through the SDK rather than the CLI, so the
   tests are self-contained; the schema cites `docs/design/persistence.md` as the
   interface it copies, like the recipe in the first phase.
3. **Skip rather than fail when the JRE is absent**, with a message naming the
   recipe that installs it. A fresh clone on a machine without the devcontainer
   should report why, not produce a stack trace.
4. **The cases above**, one test each, each naming the record whose consequence
   it pins.
5. **`just test-e2e`**, which builds the two binaries first. `just test` is left
   alone.
6. **Documents.** Draft updates to `workspace.md` (the crate, the recipe, and why
   it is outside `just test`) and `backend.md` (that the store's DynamoDB half is
   now covered, which its current text says it is not), for confirmation. This
   phase probably needs no Decision Record of its own; that is judged when it is
   built.

## Progress

To be appended.

## Verification

To be recorded. `just test-e2e` passing from a clean `target/`, and passing again
while a `just dev-api` session is running, which is what the port choice is for.

## Retirement

- [ ] Design Documents updated — `workspace.md`, `backend.md`
- [ ] Decision Records written (DR-____), if any is warranted
- [ ] Non-obvious knowledge preserved — that these tests exist to pin
      consequences the durable layer already admits to, so a failure is a
      regression in a documented property rather than in an arbitrary detail
- [ ] No durable document depends on this log
