# Introducing the AuthContext

Status: in progress
Started: 2026-08-14
Branch: main

## Request

Build the `AuthContext` boundary that DR-0024 decided and nothing yet
implements: the service is to describe its caller in its own terms, and whatever
speaks AWS's dialect is to convert into those terms in front of the service.

This is the first of the two follow-on pieces
`docs/work/2026-08-13-local-development-policy.md` deferred when it fixed the
policy. The second — reducing `crates/devgateway` to the thin adapter — is
separate work.

### Clarifications

The work reaches both ends of the wire. `crates/server` defines and reads the
`AuthContext`, and `crates/devgateway` produces it instead of
`x-amzn-request-context`. The adapter's route table, preflight handling and mode
selection are not touched here; they belong to the reduction. Changing only one
end would leave `just dev-gateway` broken between the two logs, which is why the
wire moves in one piece.

Mock authentication's selectable subject is the `AuthContext` itself. There is no
separate mock mechanism and no configuration variable: a developer sends the
header with `curl` to be one caller or another, and a request that carries
nothing is the development owner as before.

## Interpretation

**What is being asked.** DR-0024 decided the boundary and deliberately left two
things to this log: the wire format between the adapter and the service, and the
shape of the extractor. Both are now settled against real code, and the code is
written.

Concretely, four things change:

1. `crates/server` gains an `AuthContext` — its own type, carrying a subject —
   and `identity::Owner` reads that and nothing else. `RequestContext`,
   `Authorizer`, `Jwt`, the `HashMap<String, String>` and the
   `x-amzn-request-context` constant all leave the crate.
2. Absent and malformed stop being the same case. No context is the development
   owner; a context that is present and cannot be read is a refusal.
3. `crates/devgateway` writes an `AuthContext` where it currently writes an API
   Gateway request context. `context.rs` — the stringification, the `flatten`
   for list claims, the payload-2.0 base object — goes with it.
4. **The deployed path needs something that produces the `AuthContext`, and
   DR-0024 does not say what.** See below; this is the substantive open question.

**The gap in DR-0024.** DR-0024 says the conversion happens "outside
`crates/server`, in the reduced `crates/devgateway`". That answers local
development completely and the deployment not at all: `crates/devgateway` ships
nothing (`workspace.md`), and in a deployed request the only things in front of
the service are API Gateway and the Lambda Web Adapter. The adapter writes
`x-amzn-request-context` and knows nothing about an `AuthContext`. If the service
stops reading that header and nothing else fills the gap, **every deployed
request arrives with no context and is attributed to the development owner** —
the exact failure DR-0024 exists to prevent, reintroduced by its own remedy.

So this log has to decide what converts in the deployment. The proposal is API
Gateway's own request parameter mapping, on the `/api` routes:

```hcl
request_parameter {
  request_parameter_key = "overwrite:header.x-auth-context"
  mapping               = "..."   # from $context.authorizer.claims.sub
}
```

This is not a re-implementation of AWS behaviour — it is AWS's own mechanism,
configured in the component that is already in front of the service, which is
what DR-0024 asks for and what DR-0023 permits. It also disposes of the original
defect at the root: `$context.authorizer.claims.sub` is one value API Gateway
resolves, so nothing on either side ever parses a map of claims, and the
`HashMap<String, String>` failure has no place left to live. `overwrite:`
preserves the property DR-0024's security argument rests on — the edge replaces
what a caller sent on every request.

This is a decision with durable consequences and real alternatives, and it
warrants its own Decision Record. Provisionally **DR-0025**.

**What is out of scope.**

- Reducing `crates/devgateway`. `edge.rs`, the route table, the preflight, the
  three modes and `local.api_methods`'s hand-kept copy all stay as they are; only
  what the adapter hands to the service changes. That is the second follow-on
  log.
- `crates/app`. DR-0024 is explicit that nothing in the SPA changes: it obtains a
  token and attaches it as before, and which component converts it is invisible
  from the browser.
- Verifying the deployed edge. Whether the parameter mapping behaves as intended
  can only be established by an apply against real AWS, which is DR-0023's
  arrangement rather than a shortcoming of this work.
- `docs/work/2026-08-10-api-artefact-packaging.md` and
  `docs/work/2026-08-11-local-token-verification.md`. Neither is reopened.

**Assumptions.**

- The subject is the whole of the `AuthContext` today. DR-0024 says "a subject,
  and whatever else a handler genuinely uses", and no handler uses anything else.
  The type is introduced with one field rather than with fields nothing reads.
- A request that reaches the service with no context is still a developer's own,
  and still the development owner. DR-0018 is untouched, and this is what keeps a
  fresh clone working with no configuration.
- Emptiness counts as malformed, not absent. A header present with an empty value
  means something in front tried to assert an identity and had none, which is the
  malformed case rather than the absent one.
- `/health` needs no context and gets none. It is routed outside the authorizer,
  so no mapping is declared for it, and nothing there asks for an `Owner`.
- The two `cargo test` suites are the check for everything except the deployed
  mapping. Constructing an `AuthContext` in a test needs no JSON, no header and
  no knowledge of payload format 2.0 — DR-0024 names this as one of the returns
  on the boundary.

**One departure from DR-0024 worth naming.** Its table describes mock
authentication as taking its subject from "Configuration — a subject named by the
developer". This log takes it from the `AuthContext` header instead, which is a
narrowing: there is no process-wide named subject, so switching callers is a
`curl` flag rather than a restart, and a browser through trunk cannot be anyone
but the development owner. That is the same reach `Bearer alice` had under
DR-0021, and it costs no configuration surface at all. It sits inside DR-0024's
"Not decided here" latitude, but it is a departure from the word in the table and
is flagged rather than absorbed.

## Plan

1. **Settle the wire format**, which DR-0024 hands to this log. The proposal is
   `x-auth-context` carrying JSON — `{"subject":"..."}`:
   - `backend.md`, confirmed on 2026-08-13, already declares `serde_json` a
     dependency "for the `AuthContext`", so JSON is what the confirmed design
     assumes.
   - It gives the malformed case real substance. A scalar header is malformed
     only when empty; JSON is malformed whenever it does not parse or carries no
     subject, and the absent-versus-malformed split is what DR-0024 is *for*.
   - It extends to a second field without moving the wire.

   The risk is at the deployed end: the mapping has to interpolate a `$context`
   variable into static JSON text, and whether HTTP API parameter mapping does
   that can only be confirmed by an apply. The fallback, if it does not, is a
   scalar `x-auth-subject` header with empty meaning malformed. Deciding this
   before writing code is step one because it is the one choice both ends and the
   Terraform share.

2. **Write `AuthContext` in `crates/server`.** A `serde`-derived struct with a
   subject, plus the header constant, in `identity.rs`. Delete `RequestContext`,
   `Authorizer`, `Jwt` and the `HashMap` import in the same change — the point is
   that no AWS shape survives in the crate, and leaving one behind unused would
   be the same coupling with a warning attached.

3. **Rewrite `identity::Owner` as a rejecting extractor.** Its `Rejection` stops
   being `Infallible`. Three arms, matching `backend.md`'s table exactly: a
   readable context gives its subject, no header gives `DEVELOPMENT_OWNER`, an
   unreadable header answers `401`. Update the module documentation, which
   currently explains the adapter's header at length.

4. **Cover the three cases with tests**, and the fourth that is the whole
   argument: a header carrying a subject, no header, a header that is not JSON,
   and a header that is JSON but carries no subject. The last two must reject
   rather than fall back — that assertion is the one this work exists to make
   possible.

5. **Replace `crates/devgateway/src/context.rs`** with the `AuthContext` the
   service now reads. `stringify`, `flatten` and the payload-2.0 base object go;
   what remains is building the context from the claims the authorizer accepted.
   `edge.rs` keeps its structure — the only changes are which header it strips,
   which it attaches, and that `/health` now attaches nothing at all rather than
   a context without an `authorizer` member. Its tests move with it, including
   the two forgery tests, which are as load-bearing under the new header as under
   the old one.

6. **Add the parameter mapping to `infra/api/apigateway.tf`**, on
   `aws_apigatewayv2_route.api` and not on the health route, with a comment
   saying what it converts and why the conversion is here rather than in the
   service. `just tf-validate` is as far as this can be checked before an apply.

7. **Draft DR-0025** — what produces the `AuthContext` in the deployment. It
   carries the alternatives that were weighed and rejected: a conversion process
   inside the Lambda image alongside the server, and an AWS-specific outermost
   module kept inside `crates/server`. It also has to record what DR-0024 left
   open, since a reader arriving at DR-0024 will otherwise conclude that
   `crates/devgateway` converts in production, which it cannot.

8. **Draft the Design Document updates**, for confirmation:
   - `backend.md` — remove the dated note at the top; the crate now matches what
     the document says. State the header and its encoding.
   - `deployment.md` — the API's runtime shape and "the edge is verified here,
     not locally" both describe `x-amzn-request-context` reaching the service.
     Both change, and the routes table gains the mapping.
   - `workspace.md` — what the adapter hands the service. Its own dated note
     stays: the reduction is still outstanding.
   - `index.md` — the DR-0025 row.

9. **Report back to the policy log.** Its final retirement item is that this work
   and the reduction both exist as Work Logs. Half of that is satisfied by this
   file; the other half is the reduction, which is not opened here.

## Progress

### 2026-08-14

Log opened. Read `docs/README.md`, `docs/design/index.md`, `backend.md`,
`workspace.md`, `deployment.md`, DR-0024, the policy Work Log,
`crates/server/src/identity.rs` and `main.rs`, and `crates/devgateway`'s
`main.rs`, `config.rs`, `edge.rs` and `context.rs`.

**DR-0024 has a gap, and it is the substance of this work.** It names
`crates/devgateway` as the converter, and that crate ships nothing. Nothing in
the deployed path produces an `AuthContext`, so implementing DR-0024 literally
would attribute every deployed request to the development owner. Recorded here
rather than only in the Interpretation because it is the finding, and because the
Decision Record that answers it does not exist yet.

**A decision warranting a Decision Record**, provisionally DR-0025: what converts
AWS's shape into the `AuthContext` in the deployment. Proposed answer is API
Gateway request parameter mapping, which keeps the conversion in the component
already in front of the service and removes the claims map from both ends
entirely. Alternatives to record: a second process in the Lambda image, and an
AWS-specific module retained at `crates/server`'s outer edge.

**Knowledge that has no durable home yet**, and would be lost when this log
retires: that `overwrite:` on the mapping is what carries DR-0024's security
argument into the new arrangement. The old argument was that API Gateway
overwrites `x-amzn-request-context` on every request; if the replacement mapping
were `append:` rather than `overwrite:`, a caller could supply their own
`x-auth-context` and the service would have no way to tell. This belongs in
DR-0025.

**A risk to watch at apply time.** If the mapping's source ever fails to resolve,
API Gateway skips that mapping rather than sending an empty value — in which case
a client-supplied header could survive to the service and be believed. It cannot
happen on the `/api` routes, because the authorizer has run and `sub` is always
present, but the reasoning depends on that and should be stated where it can be
found.

Stopped here for confirmation of the Interpretation and the Plan, per the working
agreement. Nothing has been written outside this file.

**Steps 1 and 6 are superseded.** Both were written before I established where
parameter mapping actually lives, and the correction matters more than the
conclusion:

- Step 6 said the mapping goes on `aws_apigatewayv2_route.api`. It does not.
  `request_parameters` is an attribute of `aws_apigatewayv2_integration`; the
  route resource carries only `request_parameter_key`, which is request parameter
  *validation* and does something else entirely. Checked against the pinned
  provider binary (6.57.1) rather than from memory.
- That has a consequence step 6 did not anticipate. `apigateway.tf` declares **one
  integration** serving both the `/api` routes and `/health`, so a mapping placed
  on it also applies to `/health`, which runs outside the authorizer where
  `$context.authorizer.claims.sub` cannot resolve. API Gateway skips a mapping
  whose source does not resolve, so a caller-supplied header would pass through
  there. The integration is therefore split in two, the health one carrying
  `remove:`.
- Step 1 chose JSON — `x-auth-context: {"subject":"..."}` — partly because
  `backend.md` already credits `serde_json` to the `AuthContext`. At
  integration-level that requires interpolating a `$context` variable into static
  JSON text, and I cannot confirm offline that HTTP API mapping supports
  interpolation rather than only a whole-value expression. A failure would surface
  at apply, after both ends were written against it. The wire is a scalar
  `x-auth-subject` instead, whose mapping value is exactly
  `$context.authorizer.claims.sub`. The service now parses nothing at all, which
  is a better outcome than the one the step was reaching for: the claims map
  disappears from both ends rather than being replaced by a smaller parse.

**A hole the scalar header opened, and the second header that closes it.** With
one header, a mapping that failed to resolve would simply omit `x-auth-subject`,
the service would read that as "no context", and the request would be attributed
to the development owner — the silent misattribution this whole log exists to
remove, reintroduced by its own remedy at the one place it matters. So the edge
also sets a static `x-auth-edge`, and the service distinguishes "nothing in front
spoke" from "something in front spoke and said nothing useful". This is not in
DR-0024 and belongs in DR-0025.

It cannot arise on the `/api` routes as they stand — the authorizer has run and a
Cognito token always carries `sub` — but that is a property of the current
configuration rather than of the design, and DR-0024's whole argument is that this
class of failure should be structural rather than argued.

**Built, and the plan held with two additions.** `crates/server` reads the two
headers and parses nothing; `crates/devgateway` produces them and `context.rs` is
deleted; the mapping and the split integration are in `apigateway.tf`; DR-0025 is
written and the four Design Documents are updated.

Two things emerged during the work that the plan did not contain:

- **`crates/server` no longer depends on `serde` or `serde_json`.** Both were
  declared for `identity.rs` alone — the comment above them in `Cargo.toml` said
  so — and nothing else in the crate names either. Removing them is the clearest
  evidence available that the coupling DR-0024 removed from the source is really
  gone, and it is recorded in DR-0025's Consequences.
- **The apply and the deploy have a required order, and the wrong one is
  dangerous.** `just tf-apply api` must precede `just deploy-api`. Applying first
  is safe: the edge sets two headers the old binary ignores. Deploying first is
  not: the new binary would find no `x-auth-edge` anywhere, read every request as
  "no edge spoke", and attribute every user's writes to the development owner
  with a 200. That is the failure this whole log exists to remove, reachable
  through a deployment ordering rather than through a claim shape. It is now a
  constraint in `deployment.md`; nothing enforces it.

**The Design Document updates were confirmed** on 2026-08-14, including the two
judgements that were mine rather than the plan's: dropping `serde` and
`serde_json` from `crates/server`, and writing the apply-before-deploy ordering
into `deployment.md` as a constraint rather than leaving it in this log.

## Verification

Run on 2026-08-14. Everything but the deployed mapping passed.

- **`just test`** — 52 pass, 0 fail. Five `identity` cases, including the two
  refusals that were inexpressible before this work, and 38 in `devgateway`
  including both forgery tests and a new one for the probe.
- **`just lint`** — clean for the host and for `wasm32-unknown-unknown`, warnings
  denied. `just fmt` applied.
- **`just dev-api` with `curl`**, all cases as designed:

  | Request | Result |
  | --- | --- |
  | No headers | `201`; sees only the development owner's items |
  | `x-auth-edge` + `x-auth-subject: alice` | `201`; lists `alice-only` |
  | `x-auth-edge` + `x-auth-subject: bob` | `[]` — the isolation check, with no token, no adapter and no credentials |
  | `x-auth-edge` alone | `401 Unauthorized` |
  | `x-auth-edge` + empty `x-auth-subject` | `401 Unauthorized` |
  | `x-auth-subject: alice` with no `x-auth-edge` | the development owner's items, not alice's |

- **`dev-api` and `dev-gateway` together**, through :3001: `Bearer carol` created
  and listed her own item, `Bearer dave` saw `[]`, no token was `401` at the
  adapter, and `/health` answered `ok`. The forgery check is the one worth
  naming — sending `x-auth-edge: forged` and `x-auth-subject: attacker` *with* a
  valid `Bearer carol` returned carol's items, and the `attacker` partition was
  empty afterwards, so the forged headers were replaced rather than honoured.
- **`terraform validate`** for the `api` layer — valid. `just tf-validate` could
  not be used: it fails on the `bootstrap` layer because this machine's AWS SSO
  session has expired, which predates this work and is unrelated to it.
- **The deployed mapping — not run.** It needs an apply, and this is the one
  check that cannot happen locally; DR-0023 is why that is acceptable rather than
  a gap. The check is that a `/api` call is attributed to the token's `sub` and
  that `GET /health` still answers `ok` — **and the apply must precede
  `just deploy-api`**, for the reason recorded above.

## Retirement

- [x] Design Documents updated — `backend.md` (the two headers, the three cases,
      the dropped `serde` dependencies; its dated note removed, since the code now
      matches it), `deployment.md` (the mapping, the split integration, the apply
      ordering constraint), `workspace.md` (what the adapter writes; its own note
      narrowed to the reduction that is still outstanding), `persistence.md` (one
      stale DR-0017 citation), `index.md` (the DR-0025 row, checked against
      `docs/decisions/` by diff). Drafted, applied, and confirmed on 2026-08-14,
      which `docs/README.md` requires before the work counts as complete
- [x] Decision Records written — DR-0025, the edge produces the `AuthContext` by
      request parameter mapping
- [x] Non-obvious knowledge preserved — the gap in DR-0024 and why it was a gap
      rather than an oversight (DR-0025 Context); why `overwrite:` rather than
      `append:` is what carries the security argument, and why the second header
      exists at all (Decision); that mapping is integration-level, which is what
      forces the split integration and what ruled out the JSON wire
      (Decision, Alternatives); the three rejected alternatives (Alternatives);
      the apply-before-deploy ordering and why the reverse is silent rather than
      loud (`deployment.md` constraint, and Progress above)
- [ ] No durable document depends on this log
- [ ] Follow-on still open — reducing `crates/devgateway`, which is the second
      piece the policy log deferred and is not started. `workspace.md` carries the
      dated note saying so
