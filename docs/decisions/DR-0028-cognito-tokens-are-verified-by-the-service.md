# DR-0028: Cognito tokens are verified by the service, not by API Gateway's authorizer

Status: accepted
Date: 2026-08-16

## Context

DR-0017 and DR-0024 twice refused to verify Cognito tokens inside
`crates/server`, both times because API Gateway's JWT authorizer was assumed
to stay in front of the function — DR-0017's Alternatives:
"reimplementing the authorizer that has already run, in front of a function
nothing can reach except through it"; DR-0024's Alternatives, unchanged:
"re-implementing an authorizer that has already run... DR-0022 does this in
the stand-in is not a precedent: the stand-in *is* the thing whose job
verification is."

Both refusals rest on the authorizer remaining the enforcement point. This
record removes it. Once it is gone, there is nothing left for in-service
verification to be redundant with, and DR-0025's parameter mapping — the
mechanism that converted the authorizer's output into the `AuthContext`
DR-0024 defined — has nothing left to convert.

The verification logic itself does not need to be invented. DR-0022 built
and proved it — JWKS fetch, RS256 signature check, `iss`/`exp` and the
`client_id`-or-`aud` audience rule — inside `crates/devgateway`'s `cognito`
mode, a stand-in whose only reason to exist was standing in for the
authorizer. Once the authorizer is gone, so is that reason.

DR-0023's principle — that AWS behaviour is not re-implemented locally — is
not disturbed by moving this logic into the service. DR-0023's own text
already names fetching a real pool's real JWKS and verifying a real
signature against it as *not* re-implementation, because it is the real
protocol rather than a second telling of AWS's specification. Only *where*
it runs changes.

## Decision

`crates/server` verifies Cognito access and id tokens itself.
`aws_apigatewayv2_authorizer.cognito` and the DR-0025 parameter mapping are
removed from `infra/api/apigateway.tf`; `crates/devgateway` is retired in
full, since verifying deployed configuration against a stand-in authorizer
is no longer a distinct question from verifying the service.

**`identity::Auth`, a 2-variant enum chosen once at startup**, mirroring
`store::Store`'s "exactly two, chosen once" shape:

- `Auth::Cognito(cognito::Verifier)` — a real token, verified against the
  pool. Selected when `COGNITO_ISSUER` and `COGNITO_AUDIENCE` are both set;
  the key set is fetched before the listener binds, exactly as
  `crates/devgateway`'s `cognito` mode fetched it, for the same reason: a
  pool that cannot be reached is a reason to stop, with the reason on
  screen.
- `Auth::Mock` — the two headers (`x-auth-subject`, `x-auth-edge`) DR-0024
  introduced for header-based local development, unchanged. Selected when
  both variables are unset, which is `just dev-api`'s default and every
  environment before this record.

Exactly one variable set is refused outright at startup, not treated as
unset: a partial configuration must never silently select `Mock`, which
would downgrade a deployed function to header-trusting mode on a typo — the
exact failure this record removes structurally.

**DR-0018's "absent means the development owner" rule is scoped to
`Auth::Mock` alone.** Under `Auth::Cognito`, every failure — no token, a bad
signature, wrong issuer or audience, an expired token, a token with no
usable `sub` — is refused, with no fallback. This is not new behaviour; it
is `crates/devgateway`'s already-proven `cognito` mode ("every path needs a
token, `Bearer alice` does not work") moved from a stand-in process into the
service.

**The safety argument changes shape.** Before this record, the two headers
were safe because API Gateway's parameter mapping overwrote them on every
request. That mechanism is gone. What replaces it: `Auth::Mock` is the only
variant that ever reads those headers, and it is only the active variant
when `COGNITO_ISSUER`/`COGNITO_AUDIENCE` are unset — which the deployed
Lambda never has, since Terraform always sets both, mirroring `TABLE_NAME`.
The deployed function is therefore always `Auth::Cognito`, which never reads
either header at all. The property now depends on which enum variant was
chosen at startup, not on anything upstream — the same shape `Store`'s own
safety property already has for the data layer.

**`infra/api/apigateway.tf`'s `local.api_methods` stays enumerated; `ANY` is
not adopted.** DR-0009 gave two reasons for avoiding a single `ANY` route:
the JWT authorizer would intercept `OPTIONS` ahead of the HTTP API's own
preflight answer, and an HTTP API answers preflight itself only for an
*unrouted* `OPTIONS` — an `ANY` route matches it regardless of whether an
authorizer sits behind it. The first reason is gone with the authorizer; the
second is not. Adopting `ANY` now would proxy every preflight to a function
with no `OPTIONS` handler, and fixing that would mean adding a CORS layer to
`crates/server` — exactly what DR-0009 rejected for reasons unrelated to the
authorizer (the allowed origin belongs in the layer that reads it from SSM;
`crates/server` stays usable by any future client; local development stays
single-origin). DR-0009 itself is not superseded; only one of its two
supporting arguments is gone, and the conclusion is unchanged.

**The two integrations collapse to one.** With no parameter mapping left to
differ between `/api` and `/health`, nothing distinguishes them; one
integration serves both routes, and `crates/server`'s own `/health` handler
stays unauthenticated by not naming `Owner`, exactly as `/api/dashboard` now
must (see Consequences).

## Alternatives

**Keep `crates/devgateway` as a standalone local-verification convenience,
independent of whether the authorizer exists.** Rejected: DR-0022's own
Consequences already named the condition under which reversing it costs
nothing — "the service, the SPA and the infrastructure are unchanged by its
existence" — and that was true only because it stood in for something. Once
verification is the service's own, `devgateway` verifies nothing a developer
cannot already ask `crates/server` itself.

**Let `Auth::Cognito` also accept the two `Auth::Mock` headers as an
operational override.** Rejected: this reopens exactly the vulnerability
this record closes — a caller that can reach the Lambda through API Gateway
could set both headers by hand once no edge component overwrites them.
There is no operational need for it that `Auth::Mock` running on a
developer's own machine does not already cover.

**Switch to a single `ANY` route now that the authorizer is gone.** Covered
above; rejected on the surviving half of DR-0009's own argument.

## Consequences

**DR-0025 is superseded in full.** Nothing of the parameter mapping survives
as a live description of the system: no mapping, no split integration, no
`overwrite:`/`remove:` distinction.

**DR-0024 is narrowed, not superseded.** Its placement claim — "the
conversion happens outside `crates/server`, in the reduced
`crates/devgateway`" — reverses: the conversion now happens inside
`crates/server`, because there is no longer an edge component to hold it.
Its stronger claim survives unchanged: the service still defines
`AuthContext` and reads only that; nothing downstream of verification needed
to change, and nothing did.

**DR-0022 is superseded in full.** Its entire subject — a stand-in verifying
tokens on the service's behalf — no longer exists in any form. The
verification logic it built (JWKS fetch, RS256, the audience rule) survives,
ported into `crates/server`'s `jwks.rs`/`cognito.rs`, but the record's
subject was the stand-in, not the algorithm.

**DR-0023 is narrowed.** One sentence in its Decision section — "Inside the
application, AWS-specific credential material is not handled directly. It is
converted into a common `AuthContext` first" — is now stale as a placement
detail: `crates/server` does handle a raw JWT directly now. This does not
violate DR-0023's actual principle. DR-0023's own "line it draws" already
names real-JWKS-fetch-and-verify as protocol, not AWS-behaviour-restated;
what it required was separation from AWS's *specification*, not from AWS's
*material* running in a second process. The principle holds; one sentence
describing where it used to apply does not.

**DR-0010 is narrowed.** Its claim that "`crates/server` is not involved"
and its rejected alternative "Validating the JWT in `crates/server` as
well" — reasoned there as duplication against an enforcement point already
in place — are superseded by this record on the same grounds DR-0017 and
DR-0024 are: no enforcement point remains for verification to duplicate.
DR-0010's actual subject, the hand-written PKCE flow in `crates/app`, is
untouched and stays accepted.

**`crates/server/src/dashboard.rs` needed a one-line fix as part of this
record, and it is worth naming why.** Gating used to be a property of the
*route* — every `/api` route sat behind the authorizer regardless of what a
handler asked for. Gating is now a property of the *handler* — only a
handler that names `Owner` is checked at all. `dashboard()` named no
extractor, because until this record the route in front of it did the
checking. It now takes `Owner(_owner): Owner`, unused, matching
`action_types.rs`'s handlers. A handler added later that forgets to name
`Owner` is open to anyone with no token — this is now the shape the mistake
takes, and it is worth stating plainly rather than leaving implicit.

**A one-time reversal of the deploy-ordering rule was needed for the
transition, and does not recur.** The project's standing rule is `terraform
apply` before a deploy. Applying the new Terraform (authorizer and mapping
removed) while the *old* binary was still running would have been a silent
data-isolation incident: the old binary unconditionally trusted the two
headers, nothing would set them once the mapping was gone, and every request
would have read as "no edge spoke," landing in the development owner's
shared partition. The safe order was the reverse: deploy the new binary
first (it falls back to `Auth::Mock`, which the still-active old mapping
still satisfied correctly), then apply the new Terraform. This was one-time.
Once `Auth::Cognito` is the steady state, no future binary/configuration
mismatch can silently open a shared partition — every mismatch under
`Auth::Cognito` fails closed, never open — so the standing deploy-before-apply
rule resumes without exception from here.

**A single `terraform apply` could not be trusted to order its own diff
safely, which is a narrower risk than the deploy-ordering one above and was
not anticipated when this record was first written.** Applying the whole
diff in one pass twice failed with AWS's `409 ConflictException` on
`aws_apigatewayv2_authorizer.cognito` and `aws_apigatewayv2_integration.health`,
because Terraform tried to destroy both before the routes that still
referenced them had been updated — neither attempt changed anything in AWS,
confirmed by reading the routes, integrations and authorizer back directly.
Had it instead *succeeded* in that order, the risk it exposes is real: for
however long the window between "routes lose `authorizer_id`" and
"parameter mapping removed" lasted, `$context.authorizer.claims.sub` would
no longer resolve, API Gateway would skip that mapping and let a caller's
own `x-auth-subject` header through unmapped — the exact impersonation this
record exists to close, reopened mid-apply. The fix was three passes,
`-target`ed in order: `aws_lambda_function.api` alone, selecting
`Auth::Cognito` and so making the service stop trusting either header at
all; then the three routes; then everything else. **The Lambda-first pass
is what made the other two safe regardless of their own internal
ordering** — once `Auth::Cognito` was active, no header either route could
be tricked into passing through was worth anything to a caller, whatever
order the rest of the diff applied in. This is the deploy-ordering
principle above, one layer further down: putting the service in
`Auth::Cognito` first is what makes *every* subsequent step, Terraform's
internal ordering included, safe to get wrong.

**Reversing this** costs the enum, the two new files (`jwks.rs`,
`cognito.rs`), the fixture, and re-adding the authorizer and the parameter
mapping to Terraform.
