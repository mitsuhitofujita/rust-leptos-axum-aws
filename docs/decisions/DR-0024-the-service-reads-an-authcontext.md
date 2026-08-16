# DR-0024: The service reads an AuthContext, not AWS's request context

Status: narrowed by DR-0028
Date: 2026-08-13

## Context

DR-0017 decided that the service takes the caller's `sub` from the
`x-amzn-request-context` header, which the Lambda Web Adapter forwards the API
Gateway request context in. `crates/server/src/identity.rs` does exactly that:
it deserialises the header into a `RequestContext` shaped after `lambda_http`'s,
reaches `authorizer.jwt.claims`, and takes `sub` out of it.

That was the right decision for the question it answered. It ties one thing to
AWS which need not be tied, and the tie has a sharp edge.

**The claims are typed `HashMap<String, String>`**, which is correct only because
API Gateway's payload format 2.0 stringifies every claim before serialising it.
The service depends on a property of a payload format it has no other reason to
know about. DR-0021 recorded what follows if that property ever fails to hold:
one claim arriving as a number, a boolean or a list makes serde fail to decode
*the whole request context*, `subject()` returns `None`, and the request is
attributed to the development owner rather than refused. A write lands in the
wrong partition and nothing anywhere reports it.

DR-0021 existed partly to make that observable, by stringifying claims the way
API Gateway stringifies them so the shape could at least be exercised.
[DR-0023](DR-0023-aws-behaviour-is-not-reimplemented-locally.md) retracts that
rig, on the grounds that reproducing AWS's behaviour locally is the wrong cost to
carry. So the failure cannot be watched for any more, and the answer has to be
structural instead.

DR-0023 also asks for this directly: the application is to be separated from AWS
at its own edge, handling no AWS-specific credential material internally.

## Decision

**The service defines `AuthContext` and reads only that.** It is the service's
own type, describing the caller in the terms the service needs — a subject, and
whatever else a handler genuinely uses. Nothing in `crates/server` names API
Gateway, a request context, a JWT, or a claim.

**The conversion from AWS's shape happens outside `crates/server`**, in the
reduced `crates/devgateway`. This is the one principle
[DR-0021](DR-0021-the-deployed-edge-is-reproduced-outside-the-service.md)
contributes that DR-0023 carries forward, and it applies here for its original
reason: the component that speaks AWS's dialect is, in the deployment, in front
of the service, so the thing standing in for it belongs in front of the service
too.

**Both local arrangements produce the same `AuthContext`**, which is what makes
them interchangeable rather than two code paths:

| Arrangement | Produces the context from |
| --- | --- |
| Mock authentication | Configuration — a subject named by the developer, defaulting to the development owner |
| Verified tokens | A real Cognito token, verified against the pool the way the deployed authorizer verifies it (DR-0022) |

**Mock authentication takes a selectable subject.** DR-0021's `Bearer alice` and
`Bearer bob` are how the isolation `identity::Owner` provides became checkable by
hand, and DR-0023 removes the stand-in they lived in. Putting the choice into the
mock arrangement keeps the check and drops the rig: two callers, no adapter, no
tokens. An unset subject is still the constant development owner, so DR-0018's
promise — a working application out of a fresh clone with no configuration —
is unaffected.

**Absent and malformed are different, and this is the substance of the decision.**

- **No auth context at all** → the development owner. This is DR-0018 unchanged.
  Nothing in front asserted anything, so there is nothing to misread, and the
  request is a developer's own.
- **An auth context that is present but cannot be read** → the request is
  refused. Something in front asserted an identity and the service failed to
  understand it. Treating that as "nobody said anything" is how the wrong
  partition gets written, and it is the exact failure the Context describes.

DR-0018 considered rejecting a request with no header and refused it, because it
would make `just dev-api` useless without sign-in configured. That reasoning is
about the absent case and is untouched. The malformed case was not distinguished
from it at the time, and it is the one where failing closed costs nothing a
developer will ever encounter.

### Not decided here

The wire format between the adapter and the service — the header name, the
encoding, whether the context travels as JSON or as something narrower — and the
exact shape of the extractor. Those are settled by the Work Log that implements
this, where the alternatives can be weighed against real code.

## Alternatives

**Keep parsing `x-amzn-request-context` and make the claims tolerant.**
Deserialising into `HashMap<String, serde_json::Value>` and taking `sub` only if
it is a string would fix the silent misattribution without any new boundary, and
it is a two-line change. Rejected because it fixes the symptom and keeps the
coupling: the service would still know what API Gateway's request context looks
like, still hold a type shaped after `lambda_http`'s, and still be the wrong
place to ask "who is calling?" It also leaves DR-0023's requirement unmet, since
the AWS-specific material is still handled inside the application.

**Verify the token in `crates/server`.** Refused by DR-0017 and refused again.
Doing it correctly means fetching and caching the pool's JWKS and checking
signature, issuer, audience and expiry — re-implementing an authorizer that has
already run, in front of a function nothing can reach except through it. That
DR-0022 does this in the stand-in is not a precedent: the stand-in *is* the thing
whose job verification is.

**Put the conversion in `crates/shared`.** Would let the service and the adapter
agree on the type by construction. Rejected because `crates/shared` is compiled
for WASM as well as for the host and must stay free of platform-specific
dependencies — `workspace.md`'s first constraint — and because the SPA has no
business knowing how the API learns who is calling. The two crates agreeing on a
small serialised shape is cheaper than either depending on the other.

**Refuse in both cases — no context means 401.** The strictest option, and it
makes production fail closed. Rejected for DR-0018's original reason, restated:
`just dev-api` would need sign-in configured before a form could be submitted,
which is the configuration that decision exists to avoid. The absent case is not
where the danger is.

## Consequences

**DR-0017 is superseded**, and its Status line points here. Two of its three
findings survive whole:

- User isolation is one extractor in one file, and a handler that wanted to
  bypass it would have to say so in its signature. `identity::Owner` remains
  exactly that.
- The hop into the service is not a security boundary. Whatever carries the
  `AuthContext` can be forged by anyone who can reach the service directly; what
  makes that irrelevant is that nothing can, because API Gateway is the only
  route to the function and overwrites what it forwards on every request. If the
  service is ever exposed by any other path, this has to be revisited before that
  path opens.

What does not survive is DR-0017's mechanism — the header name, the
`RequestContext` type and the `HashMap<String, String>` — which moves out of the
service into the adapter.

**The load-bearing dependency on the adapter moves but does not go away.**
DR-0017 noted that a future adapter version renaming or dropping
`x-amzn-request-context` would break authentication quietly. After this, it
breaks the adapter instead of the service, where the header name is that crate's
whole subject rather than an incidental constant — and the malformed-versus-absent
split means a context the adapter fails to produce is a refusal rather than a
silent development-owner write, in the one deployment where it could matter.

**`crates/server` becomes testable for identity without any AWS shape at all.**
Constructing an `AuthContext` in a test needs no JSON, no header and no knowledge
of payload format 2.0, which is what lets the owner-isolation property be checked
by `cargo test` rather than by running three processes.

**Nothing in `crates/app` changes.** The SPA obtains a token and attaches it as
before (DR-0010); which component converts it is invisible from the browser.

**Reversing this costs the type and the conversion.** The adapter would take back
the parsing DR-0017 described, and the malformed case would return to being
indistinguishable from the absent one — which is the part worth not reversing.
