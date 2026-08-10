# DR-0018: The service runs without AWS, on an in-memory store and a development owner

Status: accepted
Date: 2026-08-10

## Context

Giving `crates/server` a store raised a question the service never had before:
what `just dev-api` does. Until now it answered `/api/dashboard` from hardcoded
values and needed nothing — no credentials, no configuration, no network. The
frontend is deliberately the same, and DR-0008 states the principle it follows:
every compile-time variable has an unset value that means something workable
rather than something broken, and that is what keeps development needing no
configuration at all.

The obvious alternative to preserving that is for development to write to the
real table, which means an AWS session before anything can be tried, and
development data in the production table. DynamoDB Local would avoid both, but
it is a container, and the development container has no Docker.

The second half of the question is identity. [DR-0017](DR-0017-the-service-reads-its-caller-from-the-adapters-request-context.md)
has the service read the caller's `sub` from a header the Lambda Web Adapter
forwards. Locally there is no adapter, no API Gateway and no authorizer, so
there is no header — and every item the service stores is keyed by an owner.

## Decision

The store is chosen at startup from the environment. `TABLE_NAME` set selects
DynamoDB; unset selects an in-memory store behind a mutex, which lives as long
as the process. Terraform sets that variable on the Lambda and nothing sets it
locally, so the deployment and the development server differ by configuration
rather than by code.

A request with no request-context header is attributed to a constant
development owner. It is an ordinary partition key, so development data sits
beside real data in the same shape without mixing with it.

Both are expressed as one enum with two variants rather than a trait with two
implementations, and the fallback is a plain `unwrap_or_else` rather than a
configurable identity.

## Alternatives

- **Write to the real table in development.** Rejected because it puts an AWS
  session in front of every local run, including the ones that only want to see
  a form submit, and because it fills a production table with test rows. It
  remains available: setting `TABLE_NAME` is all it takes.
- **DynamoDB Local.** Rejected because it needs a container runtime the
  development container does not have. It would otherwise be the better answer:
  one implementation instead of two.
- **Reject a request with no header.** Rejected because it makes `just dev-api`
  useless without sign-in configured, which is the opposite of what DR-0008 asks
  an unset value to mean.
- **Take the development owner from an environment variable, and reject when it
  is unset.** Considered seriously, because it makes production fail closed: the
  Lambda would never carry the variable, so a missing header there would be a
  401 rather than a write to the development partition. Rejected because the
  case it guards against — API Gateway invoking the function without a request
  context — is not a case that occurs, and paying for it with a variable every
  developer must set is the configuration this decision exists to avoid.

## Consequences

`just dev-api` and `just dev-web` together are a working application, out of a
fresh clone, with no credentials and no setup. That is the whole point.

Two implementations can drift, and only one of them is real. The in-memory store
answers from a `Vec` in insertion order where DynamoDB answers a `Query` in key
order; those agree only because the key embeds a ULID. Anything that changes the
key encoding has to change both, and the tests only cover the one that runs
without AWS. A behaviour that differs between them is invisible until deployment.

Data does not survive a restart in development, which is a feature until someone
forgets it.

The development owner is a real value in the real table's key space. Nothing
stops a deployed function from writing under it if the header ever goes missing
— see the consequences of DR-0017 — and nothing cleans up items stored under it
by a developer whose `TABLE_NAME` was set.
