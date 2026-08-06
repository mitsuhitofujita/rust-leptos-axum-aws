# DR-0007: The Cognito hosted-UI domain prefix does not echo the project name
Status: accepted
Date: 2026-08-04

## Context

Every name in the infrastructure derives from one `project` variable,
`rust-leptos-axum-aws`. That held for all four layers until `identity` was
applied for the first time, when Cognito refused the hosted-UI domain:

```text
InvalidParameterException: Domain cannot contain reserved word: aws
```

Cognito reserves `aws`, `amazon` and `cognito` in a user-pool domain prefix and
rejects any prefix containing one of them as a substring. The project name
contains `aws`, so `rust-leptos-axum-aws-auth` could never have worked. Nothing
short of an apply reveals this: the prefix is a plain string in the provider
schema, so `terraform validate` passes and the failure arrives only after the
user pool, the Google identity provider, the app client and three SSM parameters
have already been created.

The constraint applies to this one name. The user pool itself, both S3 buckets,
the Lambda, the HTTP API, the IAM role and every SSM path carry `aws` without
complaint — the reserved-word rule exists because the prefix becomes a public
hostname under `amazoncognito.com`, where an `aws` in the name would read as an
endorsement by AWS.

## Decision

The hosted-UI domain prefix is `rust-leptos-axum-auth` — the project name with
its `aws` segment dropped — and `hosted_ui_domain_prefix` is documented as the
one name that deliberately does not derive from `var.project`.

`var.project` is unchanged. The naming rule in `deployment.md` stands with one
stated exception rather than being weakened.

## Alternatives

**Rename the project.** Drop `aws` from `var.project` so every name stays
uniform. Rejected: the project name is the repository name and the root of every
SSM path, it is already applied across `bootstrap` and `delivery`, and it names
the state bucket — a name a `backend` block cannot interpolate. Renaming would
mean recreating the state bucket, the SPA bucket, the user pool and every
parameter, all to satisfy a rule that binds one hostname. The cost is enormous
and lands entirely outside the place with the problem.

**Derive the prefix mechanically**, with something like
`replace(var.project, "-aws", "")`. Rejected: it keeps the appearance of one
source of truth while hiding why the substitution exists, and it silently
produces a wrong answer for any future project name that carries a reserved word
somewhere other than a trailing segment. An explicit default with the reason
written next to it fails visibly instead.

**A prefix with an account-id suffix**, matching how the two buckets solve
global uniqueness. Rejected: the hosted-UI domain appears in the browser's
address bar during sign-in, so the suffix would publish the AWS account id to
every user. The buckets are private and carry no such cost.

## Consequences

The Google OAuth client's authorised redirect URI is derived from this prefix
(`https://<prefix>.auth.<region>.amazoncognito.com/oauth2/idpresponse`) and is
configured by hand in the Google Cloud console, outside Terraform. Changing the
prefix therefore always means a matching console edit, and sign-in through
Google stays broken until it is made.

Everything downstream reads the domain from
`/<project>/identity/hosted_ui_domain` rather than assembling it, so the SPA
build and the API need no change when the prefix moves.

Reversing this costs a `aws_cognito_user_pool_domain` replacement plus the same
console edit. The user pool and its identities survive it; only the sign-in
hostname changes.
