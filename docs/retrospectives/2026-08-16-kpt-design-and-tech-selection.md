# KPT Retrospective: Design and Technology Decisions

Date: 2026-08-16

## Scope

A full read of `docs/decisions/DR-0001` through `DR-0028` and `docs/design/`,
done in one sitting on 2026-08-16, with an outside opinion attached. See
`docs/README.md` for what a Retrospective is and how it relates to the other
document types.

## Keep

**The habit of recording being wrong, in the open.** DR-0018 rejected
DynamoDB Local because "it needs a container runtime the development
container does not have"; [DR-0020](../decisions/DR-0020-local-verification-runs-against-dynamodb-local.md)
found that premise false — it needs a JRE, not Docker — and said so plainly.
[DR-0021](../decisions/DR-0021-the-deployed-edge-is-reproduced-outside-the-service.md)
and [DR-0022](../decisions/DR-0022-real-cognito-tokens-are-verified-locally-by-the-stand-in.md)
built a real stand-in for API Gateway and the Lambda Web Adapter; DR-0023
concluded that reproducing AWS's specification locally has no terminus and
reversed course; [DR-0028](../decisions/DR-0028-cognito-tokens-are-verified-by-the-service.md)
went further and removed the authorizer this whole chain was standing in for.
Nothing here was hidden or quietly rewritten — each correction is its own
record, with the wrong premise stated before the fix.

**Layering infrastructure by blast radius, not by environment.**
[DR-0005](../decisions/DR-0005-infrastructure-layered-by-blast-radius.md)
noticed that the conventional prod/dev split does not apply — there is one
environment — and replaced it with a split along "what does destroying this
cost," putting the Cognito user pool (irreversible) and the Lambda (one
redeploy) in different blast radii instead of different environments. That is
a sharper read of the actual risk than the default template would have
produced.

**"Unset means a working default" applied consistently.**
[DR-0008](../decisions/DR-0008-the-spa-is-configured-at-compile-time.md) and
[DR-0018](../decisions/DR-0018-the-service-runs-without-aws.md) both commit to
this rule for the frontend and the backend respectively, and later records
keep honoring it deliberately — DR-0022 and DR-0028 both call out the one
place they depart from it (Cognito verification refuses to default) and say
why. A fresh clone runs with no credentials and no setup, and that property
survived nine decision records touching authentication.

**Decisions backed by measurement, not impression.**
[DR-0019](../decisions/DR-0019-the-icon-catalog-ships-lucide-geometry-not-lucide-components.md)
put a number on the cost it was rejecting — +1.69 MB of raw wasm from
generated Leptos components carrying five reactive props nothing varies — and
restructured around the actual measured saving rather than a guess.

**Append-only Decision Records that stay readable as the system changes.**
DR-0028 updates the Status line of five earlier records (superseded or
narrowed) without rewriting their reasoning, and correctly distinguishes
"superseded in full" (DR-0022, DR-0025) from "narrowed" (DR-0010, DR-0023,
DR-0024) — a distinction that matters and is easy to blur under time pressure.
Checked directly: `docs/design/workspace.md` already reflects `devgateway`'s
retirement and cites DR-0028 rather than retelling the story, which is the
Design Document rule working as intended.

## Problem

**Authentication churned through nine decision records in under two weeks.**
DR-0010, DR-0011, DR-0017, DR-0021, DR-0022, DR-0023, DR-0024, DR-0025,
DR-0028 all touch the same concern. A whole crate, `crates/devgateway`, was
designed, built, and extended across DR-0021 and DR-0022, then judged
unbounded maintenance in DR-0023, then removed outright in DR-0028. Each step
was well-reasoned on its own terms, but the shape as a whole suggests the
original edge (API Gateway JWT authorizer in front of an otherwise plain axum
service) was never stress-tested against "what does local verification of
this actually require" before infrastructure was built on top of it.

**Several failures were only discoverable by running `terraform apply`
against real AWS.** [DR-0007](../decisions/DR-0007-the-hosted-ui-domain-prefix-does-not-echo-the-project-name.md)'s
reserved-word rejection, [DR-0026](../decisions/DR-0026-the-api-is-packaged-as-a-container-image.md)'s
glibc mismatch (explicitly "safe by coincidence, not by anything that
enforced it"), and DR-0028's two `409 ConflictException` failures mid-migration
are three separate instances of the same pattern: `terraform validate` and
`cargo check` cannot see these classes of fault, and the only place they
surface is a live apply against the one production environment this project
has.

**Structural changes are rehearsed against production, because production is
the only environment.** DR-0005's single-environment model is a deliberate,
reasonable choice, but DR-0028's migration is a direct consequence of its
cost: reordering `apply` into three `-target`ed passes to keep every
intermediate state fail-closed was necessary *because* there was no lower-risk
environment to try the diff against first. It worked, but the margin for
getting it wrong was thin, and the same shape will recur for the next
structural infrastructure change.

**Several places rely on a human remembering to keep two things in sync, with
no detector.** [DR-0020](../decisions/DR-0020-local-verification-runs-against-dynamodb-local.md)
names the region and table name in the `justfile` as hand-maintained against
`infra/`; [DR-0019](../decisions/DR-0019-the-icon-catalog-ships-lucide-geometry-not-lucide-components.md)'s
`just icons` has nothing enforcing that it was run after a version bump;
DR-0026's `objdump -T` glibc check is a manual step, not a gate. None of these
have caused an incident yet, but each is a drift that is silent until someone
happens to exercise it — the same failure mode DR-0023 named as the reason to
stop reproducing AWS locally, recurring in smaller forms elsewhere.

## Try

**Spike before building infrastructure around an edge assumption.** The
API-Gateway-authorizer-in-front-of-a-plain-service shape drove five DRs of
supporting tooling before DR-0023/DR-0028 concluded the tooling cost more than
the shape was worth. A small throwaway spike — verify a real token against a
real pool once, by hand, before designing a crate to make that repeatable —
might have surfaced DR-0023's conclusion earlier and saved the build-then-
retract cycle on `crates/devgateway`.

**Write down the DR-0028 migration pattern as a reusable rehearsal
checklist.** The three-pass, `-target`ed apply ordering that kept every
intermediate state fail-closed is exactly the kind of knowledge `docs/README.md`
says belongs in a Decision Record rather than being lost — it already is one.
Consider lifting the general principle ("land the enforcement-tightening
change first, structural cleanup after") into `docs/design/deployment.md` as
a named procedure, so the next structural change (a schema migration, a store
swap) starts from a checklist instead of being reasoned out from scratch under
apply pressure again.

**Turn the manual glibc check into a CI gate once GitHub Actions lands.**
DR-0026 already names `objdump -T` as "the check"; DR-0027 defers CI to future
work. When that lands, promoting this specific check out of "something a
person remembers to run" and into the pipeline closes exactly the gap DR-0026
itself surfaced by not being checked.

**Take inventory of the hand-synchronized values and see which can be
generated instead of remembered.** Region, table name, and project name in
the `justfile`; the icon catalog's pinned-version-then-regenerate step. None
are urgent individually, but as a set they are the same "nothing local can
read Terraform" constraint recurring — worth one pass to see whether a
`just check-drift` recipe, or generating the `justfile` constants from
`terraform output`, removes more than one instance at once.
