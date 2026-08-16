# Design Document Index

Updated: 2026-08-16

The entry point to the durable layer. Start here, read the document covering the
area being changed, and follow the Decision Record citations when a constraint
looks arbitrary. See `docs/README.md` for how these documents relate to Work
Logs and Decision Records.

## The system in one paragraph

A web application: a client-side rendered Leptos single-page application,
compiled to WebAssembly and delivered as static files, talking to a separate
axum HTTP API. Both are Rust crates in one Cargo workspace, sharing the types
that cross the boundary. It deploys to AWS: the bundle to S3 behind CloudFront,
the API to Lambda behind an API Gateway HTTP API, with Cognito holding the
identities and one DynamoDB table holding the data.

## Documents

| Document | Covers |
| --- | --- |
| [workspace.md](workspace.md) | Crate layout, dependency management, toolchain, task runner |
| [frontend.md](frontend.md) | The Leptos SPA: routing, data fetching, build, assets |
| [backend.md](backend.md) | The axum service: endpoints, caller identity, the store it chooses |
| [visual-design.md](visual-design.md) | Mobile shell, design tokens, typography, surfaces, motion, accessibility |
| [page-layouts.md](page-layouts.md) | Screen inventory, information hierarchy, and navigation intent |
| [persistence.md](persistence.md) | The DynamoDB table: key encoding, item attributes, the query behind each screen |
| [deployment.md](deployment.md) | AWS runtime, the Terraform layering, artefact deployment |

Not yet written, because it does not exist yet:

- **ci** — the pipeline that runs the Terraform applies and pushes the
  artefacts. Both halves exist as commands a person runs: `infra/` holds the
  configuration, and `just deploy-web` / `just deploy-api` push the artefacts.
  All five layers have been applied and both artefacts deployed, by hand.
  Nothing automates any of it, and a document belongs here once something does.

## Decision Records

| Record | Decision |
| --- | --- |
| [DR-0001](../decisions/DR-0001-csr-spa-with-separate-api.md) | CSR SPA served as static files, with a separate API |
| [DR-0002](../decisions/DR-0002-stay-on-the-leptos-0-8-line.md) | Build on the Leptos 0.8 line, not the 0.9 prerelease |
| [DR-0003](../decisions/DR-0003-trunk-as-a-pinned-prebuilt-binary.md) | trunk as a pinned prebuilt binary; wasm-bindgen left to trunk |
| [DR-0004](../decisions/DR-0004-terraform-as-the-iac-tool.md) | Terraform is the Infrastructure as Code tool |
| [DR-0005](../decisions/DR-0005-infrastructure-layered-by-blast-radius.md) | Infrastructure layered by blast radius, not by environment |
| [DR-0006](../decisions/DR-0006-state-backend-configured-from-outside-the-repository.md) | The Terraform state backend is configured from outside the repository |
| [DR-0007](../decisions/DR-0007-the-hosted-ui-domain-prefix-does-not-echo-the-project-name.md) | The Cognito hosted-UI domain prefix does not echo the project name |
| [DR-0008](../decisions/DR-0008-the-spa-is-configured-at-compile-time.md) | The SPA receives its configuration at compile time, not at runtime |
| [DR-0009](../decisions/DR-0009-cors-is-answered-by-the-http-api.md) | CORS is answered by the HTTP API, so `/api` routes are declared per method |
| [DR-0010](../decisions/DR-0010-the-spa-signs-in-through-the-hosted-ui-by-hand.md) | The SPA signs in through the hosted UI, with a PKCE flow written by hand — narrowed by DR-0028 |
| [DR-0011](../decisions/DR-0011-authenticated-screens-are-guarded-in-the-router.md) | Authenticated screens are guarded in the router, as experience rather than enforcement |
| [DR-0012](../decisions/DR-0012-action-types-choose-icons-from-a-built-in-set.md) | Action types choose one icon from an application-owned built-in set — superseded by DR-0014 |
| [DR-0013](../decisions/DR-0013-action-type-icons-use-a-searchable-modal-picker.md) | Action type icons are selected through a searchable modal picker |
| [DR-0014](../decisions/DR-0014-action-type-icons-use-lucide-names-and-svgs.md) | Action type icons use Lucide canonical names and locally rendered SVGs |
| [DR-0015](../decisions/DR-0015-one-dynamodb-table-keyed-by-owner-and-entity-kind.md) | One DynamoDB table holds every entity, keyed by owner and entity kind |
| [DR-0016](../decisions/DR-0016-records-copy-their-action-types-display-attributes.md) | An action record copies its action type's display attributes |
| [DR-0017](../decisions/DR-0017-the-service-reads-its-caller-from-the-adapters-request-context.md) | The service reads its caller from the adapter's request context — superseded by DR-0024 |
| [DR-0018](../decisions/DR-0018-the-service-runs-without-aws.md) | The service runs without AWS, on an in-memory store and a development owner |
| [DR-0019](../decisions/DR-0019-the-icon-catalog-ships-lucide-geometry-not-lucide-components.md) | The icon catalog ships Lucide geometry, not `lucide-leptos` components |
| [DR-0020](../decisions/DR-0020-local-verification-runs-against-dynamodb-local.md) | Local verification runs against DynamoDB Local, pinned in the development image |
| [DR-0021](../decisions/DR-0021-the-deployed-edge-is-reproduced-outside-the-service.md) | The deployed edge is reproduced locally, outside the service — superseded by DR-0023 |
| [DR-0022](../decisions/DR-0022-real-cognito-tokens-are-verified-locally-by-the-stand-in.md) | Real Cognito tokens are verified locally, by the stand-in and not by the service — superseded by DR-0028 |
| [DR-0023](../decisions/DR-0023-aws-behaviour-is-not-reimplemented-locally.md) | AWS behaviour is not re-implemented locally — narrowed by DR-0028 |
| [DR-0024](../decisions/DR-0024-the-service-reads-an-authcontext.md) | The service reads an `AuthContext`, not AWS's request context — narrowed by DR-0028 |
| [DR-0025](../decisions/DR-0025-the-edge-produces-the-authcontext-by-parameter-mapping.md) | The edge produces the `AuthContext` by API Gateway request parameter mapping — superseded by DR-0028 |
| [DR-0026](../decisions/DR-0026-the-api-is-packaged-as-a-container-image.md) | The API is packaged as a container image, with the Lambda Web Adapter built in as an extension — narrowed by DR-0027 |
| [DR-0027](../decisions/DR-0027-the-devcontainer-reaches-the-hosts-container-engine.md) | The devcontainer reaches the host's container engine over its socket |
| [DR-0028](../decisions/DR-0028-cognito-tokens-are-verified-by-the-service.md) | Cognito tokens are verified by the service, not by API Gateway's authorizer |
