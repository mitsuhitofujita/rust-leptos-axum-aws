# DR-0026: The API is packaged as a container image, with the Lambda Web Adapter built in as an extension
Status: accepted — the host-only build/push claim in Decision and Consequences is narrowed by DR-0027
Date: 2026-08-15

## Context

The deployed function answered 500 to every request. Its log showed the
dynamic loader refusing `/var/task/bootstrap` before `main`:
`/lib64/libc.so.6: version 'GLIBC_2.38' not found`, and again for
`GLIBC_2.39`. `provided.al2023` ships glibc 2.34; the zip form built the
binary natively in the devcontainer, whose glibc is 2.41.

Two independent dependencies introduced the gap, both by way of the AWS SDK
reaching further into the system than before: `aws-lc-sys` compiled C code
locally against the devcontainer's own glibc headers, redirecting `strtol` and
`sscanf` to `GLIBC_2.38` symbols the runtime's glibc does not have; separately,
`std::process`, linked in for `credential_process` support, acquired a weak
reference to `GLIBC_2.39`'s `pidfd_*` family at this project's own link step.
Neither library is defective — the same sources, compiled on the runtime's own
glibc, would not have required either symbol. Before the SDK, the binary's
highest versioned symbol was `GLIBC_2.34` exactly: the native build was safe
by coincidence, not by anything that enforced it, and `docs/design/deployment.md`
had already recorded that coincidence as the entire basis of its safety.

## Decision

The artefact stops inheriting the development environment. `crates/server` is
now built inside a container image, on `public.ecr.aws/lambda/provided:al2023`
in both of `infra/api/Dockerfile`'s stages, so the binary never links against a
glibc newer than the one the function ships — the gap closes by construction
rather than by a coincidence nothing checks. The Lambda Web Adapter is copied
into the image from its own published image,
`public.ecr.aws/awsguru/aws-lambda-adapter:1.0.1`, as an extension at
`/opt/extensions/lambda-adapter`, replacing the layer attachment the zip form
used. The image's `ENTRYPOINT` is the service binary directly; there is no
`AWS_LAMBDA_EXEC_WRAPPER`, since nothing needs to redirect the entry point to
an adapter that is no longer a layer.

The image is built and pushed from the host, not the devcontainer. The
devcontainer has no container engine, and deployment is intended to move to
GitHub Actions later — building this into the devcontainer, or standing up
AWS CodeBuild now, would both be work a CI runner replaces once that lands.

## Alternatives

**Cross-compile with `cargo-zigbuild`, targeting
`x86_64-unknown-linux-gnu.2.34`.** zig supplies matching glibc headers and
stubs to both the C compilation and the final link, so it closes both symbol
families in one step without changing where or how the binary is built.
Rejected before `cargo-zigbuild` was even installed: the role a second
toolchain would play in closing this gap was judged less convincing than
separating the artefact from the development environment outright.

**Pin the devcontainer to `amazonlinux:2023`,** matching the runtime's glibc so
the existing native, zip-packaged build becomes safe again. Rejected: it closes
the gap by coupling the development environment to the runtime's glibc, so
every future development tool would have to run on it too. `trunk`'s prebuilt
`x86_64-unknown-linux-gnu` binary already requires `GLIBC_2.35` — one version
past `amazonlinux:2023`'s 2.34 — which is exactly this cost arriving early,
not hypothetically.

**Build a static `x86_64-unknown-linux-musl` binary,** kept in the existing zip
plus layer form. Rejected in favour of the container image because it leaves
the coupling risk open for the next dependency — a musl target removes today's
glibc contract but establishes no structural reason a future one could not
reappear in some other form — and because `aws-lc-sys` under musl was an open
risk (possibly needing `bindgen` and `libclang`) against a change that does not
even remove the layer-and-zip packaging it was meant to simplify.

**Base the runtime image on `public.ecr.aws/lambda/provided:al2023` while still
building the binary in the devcontainer, then copying it in.** Considered and
rejected explicitly, as a trap rather than a real option: a container image
only fixes the glibc gap if the binary is compiled inside it. Copying in a
binary built elsewhere reproduces the exact failure this record exists to
close.

## Consequences

**What this makes easy.** The binary's glibc requirement is bounded by
construction, not by a fact someone has to remember to keep true. `objdump -T`
over the binary inside the built image, filtered for `GLIBC_`, is still the
check, and it is now checking a build property rather than an environmental
coincidence.

**What this makes hard, or at least different.** `just deploy-api` no longer
runs inside the devcontainer — it needs `docker` (or a docker-CLI-compatible
engine) and now runs on the host. A cold start carries a larger image than a
7&nbsp;MB zip, though nothing has measured the difference yet. The migration
itself replaces the running function rather than updating it in place —
`package_type` cannot change on an existing function — so, unlike every other
ordering constraint `docs/design/deployment.md` records, this one apply is not
the zero-downtime kind; it needs an image already pushed to the `api` layer's
ECR repository before that apply, since there is no placeholder image
equivalent to the zip form's committed stub.

**What this would cost to reverse.** Reverting to a zip plus a Lambda layer is
symmetric: package the binary as a zip again, point the function back at
`provided.al2023`/`bootstrap`/the adapter layer ARN, and the same
`package_type` replacement happens in the other direction. Nothing about this
decision is a one-way door beyond that replacement.
