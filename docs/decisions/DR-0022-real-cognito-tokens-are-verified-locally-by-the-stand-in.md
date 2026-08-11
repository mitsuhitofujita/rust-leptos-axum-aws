# DR-0022: Real Cognito tokens are verified locally, by the stand-in and not by the service

Status: accepted
Date: 2026-08-11

## Context

DR-0021 put `crates/devgateway` in front of the unmodified service and reproduced
five behaviours of the deployed edge. It stopped short of the sixth. Its
authorizer decodes a token and does not verify it, which exercises everything
downstream — the request context, the subject, the isolation `identity::Owner`
provides — and says nothing about whether `aws_apigatewayv2_authorizer.cognito`
would have accepted the same token.

That gap is not the service's. The service is indifferent to it: DR-0017 records
that verification belongs to the component in front, and DR-0018 that the service
runs without AWS at all. The gap belongs to `infra/api/apigateway.tf`, whose
`jwt_configuration` is four lines with a small number of ways to be subtly wrong:

| Way it can be wrong | How it surfaces today |
| --- | --- |
| `issuer` is not the pool's issuer URL exactly | every call 401s after an apply |
| `audience` names the wrong app client | every call 401s after an apply |
| The SPA sends the id token where the access token was meant, or the reverse | depends on which claim the audience is checked against |

Every one of them arrives as a 401, and a 401 is also what a broken sign-in, an
expired token, a missing `Authorization` header and a wrong callback URL produce.
There was nothing to tell them apart, and nothing that could be run before the
apply that would have predicted any of them.

The third row is the one that is genuinely easy to get wrong, because the claim
carrying the app client id depends on which token was sent. A Cognito **access**
token carries it as `client_id`; an **id** token carries it as `aud`. API Gateway
accepts either. A stand-in checking only `aud` would refuse what the deployment
accepts; one checking only `client_id` would accept what it refuses. Both
mistakes are silent, and both invert the value of having a stand-in at all.

This decision was taken as the third of four phases in a piece of work
establishing local verification of the deployed system, after DR-0020 did the
same for the DynamoDB table and DR-0021 for the edge.

## Decision

A third mode, `cognito`, in `crates/devgateway`. It is `local` in every respect —
the same route table, the same preflight, the same discarding of an inbound
`x-amzn-request-context`, the same request context on the way out — with one
difference: the authorizer reaches a verdict the way the deployed one does.

It fetches the pool's key set from `{issuer}/.well-known/jwks.json`, which is the
document API Gateway reads and which is public, so the mode needs no AWS
credentials to verify anything. Credentials are needed to *learn* the issuer and
the app client id, which `just dev-gateway-cognito` resolves from the same SSM
parameters the `api` layer reads them from — so the mode checks the deployed
configuration rather than a transcription of it.

Then, in order: RS256 against the key the token's `kid` names; `iss` against the
configured issuer, exactly; `exp`, and `nbf` when present; and the audience,
**satisfied by `client_id` or by `aud`**, the latter as a string or an array.
Either satisfies. That rule is the point of the mode and is written out in
`check_audience` with the reason attached.

**Every refusal is `401 {"message":"Unauthorized"}` and nothing else**, which is
what the deployed authorizer answers. The reason — which check failed, what was
expected, what arrived, and which kind of token it was — is printed on the
stand-in's own terminal. The reason is what a developer needs and what a caller
must not have, and the split is what lets the rig be honest about the response
while still being useful.

**An accepted token is logged too**, naming its `token_use` and which claim
satisfied the audience. "The SPA is sending the id token" and "the SPA is sending
the access token" are the two states this mode was built to tell apart, and a 200
looks identical in both.

**The key set is fetched once, before the listener binds.** A `kid` outside the
set is a refusal and never a second fetch, so a token signed by something else
cannot turn into one request to Cognito per attempt; a pool that has rotated its
keys since the process started means restarting it, which is the right cost for
something a developer runs in the foreground. Fetching eagerly is also what keeps
`edge::decide` synchronous.

**The mode is never the default and the two values it needs have no defaults**,
which is the one place `crates/devgateway` departs from DR-0018's and DR-0008's
rule that an unset value means something workable. A defaulted issuer would
verify against the wrong pool and refuse every real token; a defaulted audience
would accept what the deployment refuses. Both failures look exactly like the
misconfiguration the mode exists to catch, so an unset value is refused at startup
where the reason is still obvious.

Verification uses `aws-lc-rs` and the JWKS fetch uses `hyper-rustls`, both already
in `Cargo.lock` beneath `aws-sdk-dynamodb`. Declaring them adds two dependency
edges and no packages, and the devcontainer image needs nothing new — the same
property DR-0021 secured for `hyper-util`.

## Alternatives

**`jsonwebtoken`.** The standard crate for this, and the first thing to reach for.
Rejected on what it would have bought: `pem`, `simple_asn1`, `num-bigint` and
`num-traits` are new packages, and it pulls `ring` beside the `aws-lc-rs` this
workspace already builds, so two cryptographic backends would be compiled to
verify one signature. Its audience validation would have had to be switched off
regardless, because API Gateway's rule — `client_id` or `aud` — is not its rule,
which leaves the signature check and `exp` as the whole of what was gained.
`RsaPublicKeyComponents { n, e }` takes the JWKS components in exactly the form
they arrive in and needs no ASN.1 parsing at all.

**A JWKS fetched by the `just` recipe with `curl` and passed in.** Would have kept
`crates/devgateway` free of a TLS stack entirely, which DR-0021 records as a
property worth having. Rejected because the TLS stack turned out to cost nothing —
`hyper-rustls` was already in the lock — and because it would have made `cognito`
mode unusable except through `just`, splitting the crate's configuration between
the environment and a recipe for no gain.

**Verifying in `crates/server`.** Refused by DR-0017, and this decision does not
disturb that. The refusal there is about the service; this is the thing standing
in for the component whose job verification is. `crates/server` and `crates/app`
are unchanged by this decision, which is the same property that made DR-0021's
shape worth choosing.

**Bundling a root certificate store.** `webpki-roots` would have made the fetch
independent of the image. `rustls-native-certs` was chosen instead so the trust
anchors are `/etc/ssl/certs` — the ones `curl` and the AWS CLI already use — and
so there is not a second set of roots in the repository to keep current.

## Consequences

**`Bearer alice` does not work in this mode.** DR-0021's most useful affordance —
two callers one `curl` flag apart, which is how the isolation `identity::Owner`
provides became checkable by hand — depends on the bearer value being taken at
its word, and there is nothing to verify a bare name against. The two modes are
therefore complementary rather than ranked: `local` for two callers and for
everything not about tokens, `cognito` for the question of whether a token is
good. The startup announcement says so, and a test pins it.

**The audience rule is a hand-maintained copy of API Gateway's behaviour**, as the
route table already is. If AWS changes how a JWT authorizer resolves an audience,
nothing here will notice. This is the same exposure DR-0021 accepted for
`local.api_methods` and is accepted again for the same reason: nothing local can
read Terraform, and nothing local can read AWS's implementation either.

**The mode needs credentials, the network, and a real pool**, so it cannot be part
of `just test` and is not. The tests that ship instead sign with a 2048-bit RSA
key committed under `crates/devgateway/src/testkey.der` and check against a fixed
clock, so the audience rule, the expiry, the issuer, the tampered signature and
the unknown `kid` are all pinned with no network and no AWS. The key is not a
secret and nothing trusts it; a pre-signed fixture would have been smaller but
would have frozen `exp` along with everything else.

**What this makes possible for the first time**: setting `DEVGATEWAY_AUDIENCE` to
the wrong value and watching a good token be refused — the deployed
misconfiguration, reproduced deliberately, on a machine, before an apply.

Reversing it costs the mode, one `justfile` recipe, `jwks.rs`, and the two
dependency edges. `local` and `passthrough` are untouched by its existence, and so
are the service, the SPA and the infrastructure.
