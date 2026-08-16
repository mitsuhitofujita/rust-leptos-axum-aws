# DR-0025: The edge produces the AuthContext by request parameter mapping

Status: superseded by DR-0028
Date: 2026-08-14

## Context

[DR-0024](DR-0024-the-service-reads-an-authcontext.md) decided that
`crates/server` reads an `AuthContext` of its own and that the conversion from
AWS's shape happens outside the service, "in the reduced `crates/devgateway`".

That answers local development completely and the deployment not at all.
`crates/devgateway` ships nothing — it is a workspace member that exists for the
people working on the project, like `crates/icongen` (`workspace.md`). In a
deployed request the only things in front of the service are the API Gateway HTTP
API and the Lambda Web Adapter. The adapter forwards `x-amzn-request-context` and
knows nothing about an `AuthContext`, and nothing else in the deployed path
produces one.

So implementing DR-0024 literally would leave every deployed request with no
`AuthContext` at all. Under DR-0018's rule that an absent context means a
developer's own machine, every deployed request would be attributed to the
development owner, and every user's writes would land in one shared partition.
That is a more complete version of the failure DR-0024 exists to prevent,
introduced by DR-0024's own remedy.

DR-0024 did not overlook this so much as not reach it: its Consequences discuss
"the adapter" without separating the Lambda Web Adapter from `crates/devgateway`,
and the two are the same phrase for different components. This record fills that
gap.

## Decision

**The deployed edge produces the `AuthContext` with API Gateway's own request
parameter mapping**, configured on the integration in `infra/api/apigateway.tf`:

```hcl
"overwrite:header.x-auth-subject" = "$context.authorizer.claims.sub"
"overwrite:header.x-auth-edge"    = "apigateway"
```

This is AWS's mechanism rather than a second telling of AWS's behaviour, so
[DR-0023](DR-0023-aws-behaviour-is-not-reimplemented-locally.md) permits it, and
it runs in the component that is already in front of the service, which is what
DR-0024 asks for.

It also disposes of the original defect at the root rather than relocating it.
`$context.authorizer.claims.sub` is a single value API Gateway resolves, so no
map of claims is serialised, forwarded or parsed anywhere. The
`HashMap<String, String>` whose stringification assumption DR-0024's Context is
about has no place left to exist.

**The wire is two scalar headers, not a serialised object.** DR-0024 left the
format to the implementing Work Log. A scalar mapping value is a single
documented `$context` expression; a JSON object would require interpolating that
expression into static text, which is a stronger assumption about what parameter
mapping supports and one that could only be tested by an apply, after both ends
had been written against it. The service consequently parses nothing at all.

**The second header is what makes the absent case safe.** With only
`x-auth-subject`, a mapping whose source failed to resolve would be skipped by
API Gateway, the header would simply be absent, and the service would read that
as "no edge spoke" and fall back to the development owner — the silent
misattribution again, in the one environment where it matters. `x-auth-edge`
carries no data; its presence is the assertion that something in front handled
the request. The three cases then follow structurally:

| Arrives | Owner |
| --- | --- |
| `x-auth-edge` and a non-empty `x-auth-subject` | that subject |
| Neither header | the development owner — DR-0018 |
| `x-auth-edge` with `x-auth-subject` absent or empty | nobody; the request is refused |

**`overwrite:` is load-bearing and is not an incidental choice of prefix.**
DR-0024 carried forward DR-0017's finding that the hop into the service is not a
security boundary, and that what makes it irrelevant is that API Gateway
overwrites what it forwards on every request. `append:` would leave a
caller-supplied header in place beside the mapped one, and the property the whole
arrangement rests on would quietly cease to hold.

**The integration is split in two.** Parameter mapping is an attribute of
`aws_apigatewayv2_integration`, not of `aws_apigatewayv2_route` — the route
resource carries only `request_parameter_key`, which is request parameter
validation and does something else. One integration previously served both the
`/api` routes and `/health`. Since `/health` is routed outside the authorizer,
`$context.authorizer.claims.sub` cannot resolve there, the mapping would be
skipped, and a caller's own `x-auth-*` headers would reach the service. A second
integration carrying `remove:` for both headers makes that impossible rather than
harmless by coincidence — the probe asks for no `Owner` today, and this does not
depend on it continuing not to.

**`crates/devgateway` produces the same two headers**, which is what keeps the
local arrangement and the deployed one interchangeable rather than two code
paths. It strips both on the way in for the same reason API Gateway overwrites
them.

## Alternatives

**A conversion process inside the Lambda image, in front of the service.** The
literal reading of DR-0024: ship something like `crates/devgateway` beside the
binary and have the Lambda Web Adapter proxy to it instead. Rejected because it
puts a second process in the function for a header rewrite, and because
`deploy-api` packages one `bootstrap` executable in a zip — the packaging would
have to change, for no gain over a mapping AWS already evaluates before the
function is invoked.

**An AWS-specific module retained at `crates/server`'s outer edge**, converting
`x-amzn-request-context` into an `AuthContext` before the extractor sees it.
Cheapest to write and needs no infrastructure change. Rejected because it is
DR-0024's coupling with an extra indirection: the crate would still hold a type
shaped after `lambda_http`'s, still parse the claims map, and still carry the
stringification assumption that started all of this. DR-0024 says nothing in
`crates/server` names API Gateway, a request context, a JWT or a claim, and this
would violate that in the letter as well as the spirit.

**One header, and accept that an unresolved mapping means the development
owner.** It cannot happen on the `/api` routes as configured, because the
authorizer has run and a Cognito token always carries `sub`. Rejected because
that is a property of the current configuration rather than of the design, and
the entire argument of DR-0024 is that this class of failure should be structural
rather than argued. The second header costs one static mapping.

**A JSON `x-auth-context` header.** Preferred initially, because `backend.md`
already credited `serde_json` to the `AuthContext` and because an object extends
to a second field without moving the wire. Rejected once the mapping was found to
be integration-level: it needs interpolation into static text, which is an
unverified assumption whose failure would appear only at apply. If the
`AuthContext` ever grows a second field, a second mapped header is the cheaper
change.

## Consequences

**`crates/server` no longer depends on `serde` or `serde_json`.** Both were
declared for `src/identity.rs` and nothing else in the crate names them; the
comment above them in `Cargo.toml` said as much. The coupling DR-0024 removed
from the source is now also gone from the manifest, which is the clearest
available evidence that it is really gone.

**Owner isolation is checkable by `cargo test` and by two `curl` flags.** Being
`alice` or `bob` under `just dev-api` is a pair of headers, with no token, no
adapter and no AWS credentials. DR-0024 asked for a selectable subject and named
configuration as the source; the headers are the source instead, which is a
narrowing — there is no process-wide named subject, so a browser through trunk is
always the development owner — and it costs no configuration surface at all.

**A fault in the mapping surfaces after an apply, not before.** `just
tf-validate` schema-checks the configuration and cannot evaluate a `$context`
expression. This is DR-0023's arrangement working as intended rather than a gap:
the edge is verified where it runs. The check is that a deployed `/api` call is
attributed to the token's `sub` and that `GET /health` still answers `ok`.

**The route table and `local.api_methods` are untouched.** This record changes
what the edge hands the service and nothing about which requests reach it.

**Reversing this costs the two mappings and the split integration.** The service
would return to reading AWS's request context directly, and the malformed case
would return to being indistinguishable from the absent one — which is the part
worth not reversing.
