# Reducing crates/devgateway to the thin adapter

Status: in progress
Started: 2026-08-14
Branch: main

## Request

Carry out the reduction of `crates/devgateway` that DR-0023 decided: the crate
becomes a thin adapter that converts a verified Cognito token into the
`AuthContext` the service reads, and does nothing else.

This is the second of the two follow-on pieces
`docs/work/2026-08-13-local-development-policy.md` deferred. The first —
introducing the `AuthContext` at both ends of the wire — is
`docs/work/2026-08-14-introducing-the-authcontext.md`, which is complete apart
from its own dependency on this one.

### Clarifications

Every request the adapter receives is authorized, whatever its path. No
exemption is kept for `GET /health`, so nothing of the route table survives and
the adapter makes no routing decision at all.

## Interpretation

**What is being asked.** DR-0023 retracted five of the six behaviours
`crates/devgateway` reproduces and kept one: the authorizer's actual verdict
against a real token (DR-0022). `workspace.md` and `deployment.md` already
describe the reduced crate as the current design and carry a dated note saying it
has not been carried out. This log carries it out and removes the note.

What goes, and why each is named in DR-0023's Decision or its "line this draws":

1. **The route table.** `API_METHODS` and `route()` in `edge.rs` are a
   hand-maintained copy of `local.api_methods` in `infra/api/apigateway.tf`,
   including the 404-rather-than-405 distinction that only exists to expose a
   difference between an HTTP API and an axum router.
2. **The preflight.** `preflight()`, `ALLOW_HEADERS`, `MAX_AGE`, `allow_origin()`
   and `DEVGATEWAY_ALLOW_ORIGIN` reproduce `cors_configuration`. The
   `access-control-allow-origin` echo in `proxy::forward` goes with them, and
   `forward`'s `allow_origin` parameter with it.
3. **`local` mode**, and with it the whole unverified path in `authorizer.rs`:
   `decoded()`, `claims()`, and `Bearer alice` as a caller named alice. DR-0024
   replaced that affordance with the two headers `just dev-api` now accepts, and
   `workspace.md` already states as a constraint that the adapter cannot be two
   callers.
4. **`passthrough` mode.** It exists to show the difference between the rig and
   its absence; with the rig gone there is no difference to show, and
   `just dev-api` alone is the thing it described.
5. **`Mode` itself**, and `DEVGATEWAY_MODE`. One behaviour needs no selector,
   which makes `Verifier` unconditional rather than an `Option`, and makes
   `DEVGATEWAY_ISSUER` and `DEVGATEWAY_AUDIENCE` always required — the departure
   from "an unset value means something workable" that `config.rs` currently
   confines to `cognito` mode becomes the crate's only rule.
6. **The two recipes become one.** `dev-gateway-cognito` disappears and
   `dev-gateway` resolves the two SSM values, which is exactly what
   `workspace.md`'s recipe table already describes. The justfile's long comment
   block above them describes the retracted rig and is rewritten.

What stays: `jwks.rs`, `testkey.rs`, the verification half of `authorizer.rs`
including `check_audience` — DR-0023 names it as a deliberate exception —
`proxy.rs` minus the CORS echo, the unconditional stripping of the two
`x-auth-*` headers on the way in, and the refusal that answers exactly what the
deployed authorizer answers while printing the reason on the terminal (DR-0022).

**The one thing DR-0023 does not settle, now settled by the clarification above:
what the adapter does with a request that is not an authorized `/api` call.**
Today `route()` decides — `/health` is forwarded without a token, anything
unrouted is a 404. Both are the route table, which is going. **Every request is
authorized instead**: the adapter verifies a token or answers 401, whatever the
path, because "converts a verified JWT into an `AuthContext` and does nothing
else" leaves it nothing with which to make a second kind of decision. The
consequence is that `GET /health` through :3001 becomes a 401 where it is
currently `ok`. Nothing uses it — the SPA proxies only `/api`
(`gateway_backend` in the `justfile`), and the probe exists for the deployed
target. The alternative, a single path exemption for `/health`, was rejected as
a one-line route table: it would keep the adapter diverging from the deployment
in a way only a reader of this crate could know about, which is the property
DR-0023 objects to rather than the size of the copy.

This is a divergence from the deployment, where `/health` is routed outside the
authorizer, and it is deliberate: the local adapter no longer claims to mirror
the edge's routing at all, so a request through it says only "would the
authorizer accept this token". It needs a durable home — see step 8.

**What is out of scope.**

- `crates/server`. Nothing in the service changes: it reads the same two headers,
  and this log changes only what stands in front of it locally. The
  `AuthContext` work is finished.
- `crates/app`, `infra/`, and the deployed edge. The parameter mapping DR-0025
  decided is already in `apigateway.tf` and is not touched.
- Reopening DR-0022. The verification half is retained wholesale, including the
  audience rule that is a copy of AWS's behaviour, because DR-0023 explicitly
  keeps it.
- `docs/work/2026-08-10-api-artefact-packaging.md` and
  `docs/work/2026-08-11-local-token-verification.md`. Neither is reopened, though
  the second's fourth phase is already cancelled by DR-0023.

**Assumptions.**

- No new Decision Record is expected. DR-0023 decided this reduction, named what
  goes and what is kept, and recorded the alternatives; this is its execution. If
  the `/health` question above turns out to have durable consequences beyond
  what DR-0023 says, that is the one candidate.
- The forgery tests are load-bearing and stay. Stripping the two `x-auth-*`
  headers unconditionally is what makes the local arrangement teach the same
  thing the deployed `overwrite:` teaches (DR-0025), and it is now the only
  behaviour of the adapter besides verification.
- `edge.rs` keeps its name and becomes small: strip, authorize, attach, or
  refuse. Folding it into `main.rs` would put its tests there too, and those
  tests are the record of the security property.
- The `crates/devgateway` dependency set does not change. `serde`, `serde_json`
  and `base64` are all still needed by the verification path, and `hyper-util`,
  `hyper-rustls` and `aws-lc-rs` are untouched.
- `just test` and `just lint` are the check, plus `dev-api` and `dev-gateway`
  together with a real token — the same manual check the `AuthContext` log ran,
  which is the one that exercises the reduced crate end to end.

## Plan

1. **Reduce `config.rs`.** Delete `Mode`, `mode()` and `DEVGATEWAY_MODE`; delete
   `allow_origin` and `DEVGATEWAY_ALLOW_ORIGIN`. `verification` stops being an
   `Option`. Rewrite the module documentation, whose opening claim — every value
   has a default — stops being true of the crate as a whole.

2. **Reduce `authorizer.rs`.** Delete `decoded()`, `claims()` and the
   `Authorization::Allowed` path that does not verify; `authorize` takes a
   `&Verifier` rather than an `Option`. Delete the `local` mode tests, including
   `a_bearer_value_that_is_not_a_jwt_is_the_subject_itself` and the `local` half
   of `a_token_whose_signature_was_tampered_with_is_refused`. Keep every
   verification test. Rewrite the module documentation, which is currently
   organised around the two behaviours.

3. **Reduce `edge.rs` to strip, authorize, attach.** Delete `Route`, `route()`,
   `API_METHODS`, `preflight()`, `allow_origin()`, `ALLOW_HEADERS` and `MAX_AGE`.
   No path is examined at all, per the clarification. `Outcome::Forward` loses
   its payload. Keep `attach()` unchanged — it is the conversion the crate exists
   for — and keep `answer()` for the 401.

4. **Reduce the tests in `edge.rs`** to what survives: both forgery tests, the
   accepted-token-with-no-subject test, the no-token refusal, and the refusal
   body. Delete the route-table, preflight and passthrough tests, and the probe
   test with them — `/health` is no longer a case. The fixture `send()` loses its
   `Mode` argument and always carries the fixture verifier, which means the
   surviving tests move from unverified fixtures to signed ones — this is the
   largest mechanical change in the log. Add one test asserting that a path
   outside `/api` is authorized like any other, since that is the behaviour the
   clarification chose and nothing else would record it.

5. **Reduce `main.rs` and `proxy.rs`.** `main.rs` loses the `Mode` match in
   `announce`, and its module documentation — a list of four reproduced
   behaviours — becomes a description of one. `proxy::forward` loses its
   `allow_origin` parameter and the header insertion.

6. **Collapse the justfile recipes.** `dev-gateway` becomes what
   `dev-gateway-cognito` was; the latter is deleted. Rewrite the comment block
   above them, which describes the three modes and the retracted rig, and check
   the last-comment-line rule `workspace.md` records for `just --list`.

7. **Draft the Design Document updates**, for confirmation:
   - `workspace.md` — remove the dated note at the top; the crate now matches the
     document. Check every sentence that describes the adapter against what was
     built, particularly the recipe table and the four constraints that mention
     it.
   - `deployment.md` — already describes `just dev-gateway` as the thin adapter
     with the authorizer's verdict. Verify rather than assume, especially the
     browser-through-the-adapter constraint.
   - `index.md` — no change expected unless step 8 produces a record.

8. **Give the `/health` divergence a durable home.** The adapter authorizing
   every path, where the deployment routes `/health` outside the authorizer, is
   not derivable from DR-0023 and is invisible in the reduced code — there is no
   exemption there to notice. A constraint in `workspace.md` is the lighter
   answer and probably the right one; a Decision Record is warranted only if the
   reasoning turns out to need the Alternatives section. Decided when the code is
   written and the sentence can be checked against it.

9. **Report back to the policy log.** `docs/work/2026-08-13-local-development-policy.md`
   was held open pending both follow-on pieces; this is the second. With this log
   complete, that log's last retirement item is satisfied and it can retire.

## Progress

### 2026-08-14

Log opened. Read `docs/README.md`, `docs/design/index.md`, `workspace.md`, the
`devgateway` sections of `deployment.md`, DR-0023, the `AuthContext` Work Log,
and all seven files of `crates/devgateway` plus its `Cargo.toml` and the
`justfile`'s gateway recipes.

**The durable layer already describes the reduced crate**, in `workspace.md` and
in `deployment.md` both, which narrows this work: the design does not have to be
decided, only carried out, and the check on each edit is whether the document's
existing sentence became true.

**One open question, asked and answered**: what the adapter does with a request
that is not an authorized `/api` call, now that the route table that answered it
is going. Settled as authorizing everything, which turns `GET /health` through
:3001 from `ok` into a 401 and leaves the adapter with no routing decision at
all. The knowledge worth keeping is not the choice but its consequence — the
local adapter and the deployed edge now differ on `/health`, and nothing in the
reduced crate shows it, because what shows a divergence is an exemption and
there is none.

Stopped here for confirmation of the Interpretation and the Plan, per the working
agreement. Nothing has been written outside this file.

**Built, and the plan held.** `crates/devgateway` is 1,474 lines where it was
1,960; `edge.rs` is a strip, an authorize and an attach; `config.rs` is 76 lines
holding three values. `Mode`, the route table, the preflight, the CORS echo and
the unverified path are gone, `dev-gateway-cognito` is folded into `dev-gateway`,
and the crate's test count falls from 38 to 26 — every deletion a behaviour that
is no longer reproduced rather than a behaviour that is no longer checked.

Four things emerged during the work that the plan did not contain:

- **`Config::for_test` was deleted rather than reduced.** With `decide` taking a
  `&Verifier` and nothing else, the edge tests need no `Config` at all, and a
  `#[cfg(test)]` constructor nothing calls is a warning under `-D warnings`. The
  fixture verifier is built in `edge.rs`'s tests directly, the way
  `authorizer.rs` already built it.
- **Two citations of superseded records were corrected in passing.** The
  `justfile`'s `Trunk.toml` note and `crates/devgateway/Cargo.toml`'s
  forwarding-leg comment both cited DR-0021, whose Status line points at DR-0023;
  `workspace.md` cites DR-0023 for the same two facts. Both now do too.
- **The adapter can no longer be run at all without AWS credentials and the
  network.** `local` mode was the arrangement that made `dev-gateway` usable on a
  machine with neither, and it is gone. This is a real cost of the reduction and
  DR-0023 accepted it knowingly — what it replaced is `dev-api`'s two headers,
  which need less than `local` mode did — but it is worth naming, because it is
  what makes the manual leg of the verification below unrunnable here.
- **The `/health` divergence went into `workspace.md` as a constraint, not into a
  Decision Record.** DR-0023 already decided that the route table goes; that a
  path exemption is a route table is an application of its reasoning rather than
  a new decision, and it needs no Alternatives section to be understood. What it
  did need was a durable home, since the reduced crate cannot show it — an
  absent exemption leaves no trace.

**The Design Document updates were confirmed** on 2026-08-14: `workspace.md`'s
dated note, crate-list line, Structure sentence and new `/health` constraint, and
`deployment.md`'s one phrase that implied a mode selector. Confirmed with them,
because both were judgements rather than transcriptions: that the divergence
belongs in a constraint rather than in a Decision Record, and that no new record
is warranted by this work at all.

## Verification

Run on 2026-08-14. Everything that does not need AWS credentials passed; the
manual leg cannot run on this machine.

- **`just test`** — 40 pass, 0 fail: 26 in `devgateway`, 14 in `server`. The
  `devgateway` set is the verification cases plus, over `decide` itself, both
  forgery tests, the marked-and-unnamed test, the no-token refusal, the refusal
  body, and the new one asserting that a path outside `/api` is authorized like
  any other.
- **`just lint`** — clean for the host and for `wasm32-unknown-unknown`, warnings
  denied. `just fmt` applied.
- **`just --list`** — `dev-gateway` and `dev-web-gateway` each show one summary
  line, so the comment rule survived the rewrite, and `dev-gateway-cognito` is
  gone from the listing.
- **Startup, both failure paths.** `cargo run -p devgateway` with nothing set
  stops with `DEVGATEWAY_ISSUER is unset. just dev-gateway resolves it from
  SSM.`; with an issuer naming a pool that does not exist it stops with
  `…/.well-known/jwks.json answered 404 Not Found`. The second is worth more than
  it looks: it is a real HTTPS request to real Cognito, so the JWKS leg is
  exercised end to end, and it confirms the fetch still happens before the
  listener binds.
- **The manual leg — not run.** `just dev-gateway` resolves its two values from
  SSM, and this machine's AWS SSO session has expired (`just _ssm` answers `Your
  session has expired`), which predates this work. No public issuer serves a key
  set at `{issuer}/.well-known/jwks.json` to stand in for the pool, so the
  adapter cannot be started at all here. What is outstanding, after `aws login`:
  `just dev-api` and `just dev-gateway` together, and through :3001 a real access
  token returning that caller's items, no token returning
  `401 {"message":"Unauthorized"}`, `Bearer alice` a 401 with the reason on the
  adapter's terminal, forged `x-auth-*` headers sent with a valid token returning
  the token's own items, and `GET /health` a 401 — the intended change, which is
  the one behaviour worth confirming by hand because it is the one a reader would
  not expect.

## Retirement

- [x] Design Documents updated — `workspace.md` (the dated note removed, since
      the code now matches the document; the crate-list line; a sentence in
      Structure pointing at the new constraint; the `/health` constraint itself),
      `deployment.md` (one phrase describing the adapter as having the verdict
      "switched on", which implied a mode selector that no longer exists).
      Drafted, applied, and confirmed on 2026-08-14, which `docs/README.md`
      requires before the work counts as complete
- [x] Decision Records written — none. DR-0023 decided this reduction, named what
      goes and what is kept, and recorded the alternatives; this log is its
      execution and adds no decision of its own
- [x] Non-obvious knowledge preserved — the `/health` divergence and why it
      leaves no trace in the code (`workspace.md` constraint); that the adapter
      now needs AWS credentials to run at all, where `local` mode did not
      (Progress above, and the recipe table already said so of the intended
      design)
- [ ] No durable document depends on this log
- [ ] The manual leg of the verification is outstanding, and needs `aws login`
