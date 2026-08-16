# API artefact packaging

Status: in progress
Started: 2026-08-10
Branch: main

## Request

The API and the SPA were both deployed and every request comes back as an
internal error. Find the cause and correct it.

### Clarifications

Solve it by changing the devcontainer's `Dockerfile`, not by adding a
cross-compiling tool. The reason given: the role `cargo-zigbuild` would play in
closing a glibc gap was not convincing.

The Lambda Web Adapter stays. The service is distributed as a binary and the
adapter is the shape that suits it.

The development environment and the deployed artefact are to be separated. The
artefact must not inherit a property of the machine that happened to build it.

Work pauses here to commit. This log is the record of what was found and what
was decided; nothing has been implemented yet.

Work resumes: carry out the packaging migration this log already decided on.
Deployment is intended to run through GitHub Actions eventually, but that is not
being built now. For the moment, only the Plan's first step — where the image is
built — is settled: the build happens outside the devcontainer, on the host,
rather than by mounting a container engine into the devcontainer or standing up
CodeBuild. The latter two are rejected for now rather than ruled out permanently;
GitHub Actions arriving later is the reason neither is worth building today.

The build-location question above is reopened. The devcontainer is to gain a
container-engine client that reaches the host engine over its socket —
Docker-outside-of-Docker, applied to podman since that is the host's actual
engine, rather than a nested engine (Docker-in-Docker), which was judged too
heavy for a need that is only ever reaching the one engine the host already
runs. Standing up GitHub Actions as the way `deploy-api` runs was weighed as
the alternative and judged heavier still for now: it needs new IAM/OIDC trust
infrastructure and has to encode the `tf-apply api`-before-`deploy-api`
ordering safely against a single production environment, where getting it
wrong is the misattribution this project's own Constraints section already
rates as unsafe rather than merely inconvenient. The socket-mount change goes
first; GitHub Actions stays future work, not ruled out.

## Interpretation

**What is being asked.** The deployed API answers 500 to everything. The cause
is a property of how the Lambda artefact is built, so the correction is a change
to the build and packaging path, not to `crates/server`.

The last clarification settles the shape of that correction: the artefact is to
carry its own runtime rather than borrow the devcontainer's. That rules out the
answer that was being drafted when it arrived — pinning the devcontainer to the
runtime's operating system — because that answer works by *coupling* the two.

**Out of scope.**

- `crates/server` itself. No line of the service is implicated; see the evidence
  below.
- The action-type slice this failure was discovered while verifying. Its work
  log is `2026-08-10-add-action-type-page.md`; the feature is complete and this
  is not a defect in it.
- Any change to what the API does, or to the SPA.

**Assumptions.**

- The devcontainer stays as the development environment. Separating the artefact
  means the artefact stops depending on it, not that it is replaced.
- The adapter's version and its role are unchanged. Only the way it is attached
  to the function moves, if the packaging moves.

## Findings

### 2026-08-10 — the cause

The function never reaches the handler. Its log group shows the same three lines
on every invocation, in `Phase: init` and again in `Phase: invoke`:

```text
EXTENSION  Name: lambda-adapter  State: Ready  Events: []
/var/task/bootstrap: /lib64/libc.so.6: version `GLIBC_2.38' not found (required by /var/task/bootstrap)
/var/task/bootstrap: /lib64/libc.so.6: version `GLIBC_2.39' not found (required by /var/task/bootstrap)
INIT_REPORT  Init Duration: 33.86 ms  Phase: init  Status: error  Error Type: Runtime.ExitError
```

`provided.al2023` ships glibc 2.34. The binary `just deploy-api` packaged
requires symbols from 2.38 and 2.39, so the dynamic loader refuses it before
`main`. API Gateway answers 500, and the SPA — served correctly by CloudFront,
which returns 200 — shows that 500 as an internal error on every screen that
calls the API.

**The adapter is not implicated.** It reaches `State: Ready` every time; it is a
separate executable that AWS builds and ships in the layer. What fails is
`/var/task/bootstrap`, which is this project's binary.

**Two independent sources, both fatal.**

| Symbols | Version | Comes from |
| --- | --- | --- |
| `__isoc23_sscanf`, `__isoc23_strtol` | GLIBC_2.38 | `aws-lc-sys` — C compiled locally against the devcontainer's glibc 2.41 headers |
| `pidfd_getpid`, `pidfd_spawnp` | GLIBC_2.39 | Rust `std`, linked in because the AWS SDK reaches `std::process` for `credential_process` |

Confirmed by `nm -u` over
`target/x86_64-unknown-linux-gnu/release/deps/libaws_lc_sys-*.rlib` and
`build/aws-lc-sys-*/out/libaws_lc_0_44_0_crypto.a` for the first pair, and over
the toolchain's own `libstd-*.rlib` for the second.

Neither is a library defect. The `__isoc23_*` names appear nowhere in aws-lc's
sources — glibc's headers redirect `strtol` and `sscanf` to them from 2.38
onwards, so the same source compiled on the runtime's own glibc produces
`strtol@GLIBC_2.2.5`. The `pidfd_*` pair is a weak reference inside a `std` that
is otherwise compatible back to glibc 2.17; it acquires a version requirement at
*this project's* link step, against whatever libc the linking host has.

**Both are hard requirements, despite being weak symbols.** `readelf -V` shows
`.gnu.version_r` carrying `GLIBC_2.38` and `GLIBC_2.39` with `Flags: none`. A
version need without the weak flag is fatal to the loader, so removing only one
of the two would not have produced a working function.

**The AWS SDK did not break this; it removed accidental headroom.** Before it,
the binary reached neither C code nor process spawning, and its highest versioned
symbol was `GLIBC_2.34` — exactly the runtime's. `docs/design/deployment.md`
already recorded that this was the entire basis of the native build's safety,
and that a dependency pulling in a newer symbol would fail at invocation rather
than at build. That is what happened.

### 2026-08-10 — constraints discovered while weighing the options

- **`trunk`'s prebuilt `x86_64-unknown-linux-gnu` binary requires GLIBC_2.35.**
  So a devcontainer moved to any base older than that — including
  `amazonlinux:2023` at 2.34 — cannot run the pinned trunk. A
  `x86_64-unknown-linux-musl` asset is published for the same release and is
  static, which is the way out if a base image is ever lowered.
- **The bundled AWS CLI is not a constraint.** Everything under
  `/usr/local/aws-cli` tops out at GLIBC_2.17, so the existing
  `COPY --from=amazon/aws-cli` stage works on any base considered.
- **`rust:1.96` publishes trixie, bookworm, bullseye and alpine variants**, so
  the base image is a free choice if it stays in that family.
- **The devcontainer has no container engine.** No `docker` or `podman` binary
  and no socket mounted; `container=podman` in the environment says the host
  runs podman. Any packaging that builds an image needs that access arranged
  first.
- **Cargo will not notice this class of change.** A build fingerprint does not
  include the C toolchain or the host's libc, so the `aws-lc-sys` archive
  already in `target/` would be reused after any environment change and would
  carry the 2.38 symbols with it. `cargo clean` — or at least
  `cargo clean -p aws-lc-sys` — belongs to whatever fix lands.

### 2026-08-10 — what was tried and withdrawn

`cargo-zigbuild` targeting `x86_64-unknown-linux-gnu.2.34` was the first
direction. It closes both halves at once, because zig supplies the glibc headers
and stubs for the requested version to the C compilation and to the final link
alike. It was withdrawn on the instruction above, before `cargo-zigbuild` itself
was installed.

Two container-local traces remain from that attempt, and nothing in the
repository changed:

- zig 0.15.2 is unpacked at `/home/vscode/.local/zig-x86_64-linux-0.15.2/` and
  symlinked as `/usr/local/cargo/bin/zig`. Both can be deleted.
- The devcontainer has no `xz`, so a throwaway Rust decompressor was written in
  the scratchpad to unpack zig's `.tar.xz`. It lives outside the repository. Any
  future need for `.xz` in the image is one `dnf`/`apt` package away.

### 2026-08-10 — the three ways out

The gap is not a defect of the container or of a library. It is a property of
shipping a dynamically linked glibc binary from an environment that is not the
one it runs in, and it can be closed at any of three layers.

| | zip + devcontainer on the runtime's OS | zip + static musl binary | container image + multi-stage build |
| --- | --- | --- | --- |
| How the gap closes | build environment is made equal to the runtime | no glibc contract exists | build happens inside the image that runs |
| Infrastructure change | none | none | ECR repository, `package_type`, IAM, placeholder |
| Deploy recipe | unchanged | target changes only | rewritten around build and push |
| Container engine needed | no | no | **yes** — not present today |
| Couples dev environment to artefact | **yes, tightly** | no | no |
| Cold start | unchanged | unchanged | worse; image is hundreds of MB against a 7 MB zip |
| Open risk | image rebuild, unverifiable from inside | `aws-lc-sys` under musl may need bindgen and libclang | engine access, placeholder ordering |

### 2026-08-15 — where the build runs, resolved

Work resumes on the Decision below. The Plan's first step — engine access, host
build, or CodeBuild — is settled: the build and the push run on the host,
outside the devcontainer, confirmed directly. Deployment moving to GitHub
Actions later is the stated reason neither of the other two is worth building
now; both stay open for whenever that lands. Nothing about the devcontainer
changes as a result. This is a plan resolution, not a new decision with
alternatives weighed against each other from scratch, so it is recorded here
rather than as its own Decision Record; the Decision Record this log still owes
is the packaging direction below, once implemented.

## Decision

**The artefact stops inheriting the development environment.** The direction is
a container image built in multiple stages, with the Lambda Web Adapter copied
in from its public ECR image as an extension, and `crates/server` compiled in a
build stage of that same image.

Rejected with it: pinning the devcontainer to `amazonlinux:2023`. It works, and
it was the option chosen earlier in the conversation, but it closes the gap by
coupling the two things this decision separates — every future development tool
would have to run on the runtime's glibc, which the trunk finding above shows is
already a live cost.

This decision is not yet implemented and needs a Decision Record when it is.

**A trap to carry forward:** a container image only fixes this if the binary is
built *inside* it. Basing the image on `public.ecr.aws/lambda/provided:al2023`
and copying in a binary built in the devcontainer reproduces exactly the present
failure.

## Plan

1. **Decide where images are built. Resolved: on the host, outside the
   devcontainer.** The devcontainer has no engine, and the three ways to give it
   one — mounting the host's podman socket, moving the build to CodeBuild, or
   this — were weighed against a deploy pipeline that does not exist yet:
   deployment is intended to run through GitHub Actions eventually, and standing
   up either alternative now would be work a CI runner replaces later. Nothing
   about the devcontainer changes; the build and push steps run as commands
   invoked on the host machine instead of through a devcontainer `just` recipe,
   until GitHub Actions takes them over. The deploy recipe's shape below reflects
   this. **Superseded 2026-08-15 — see the Plan addendum below.** The reasoning
   above was sound for what has shipped so far; the next round of work reopens
   it deliberately, on different grounds.
2. **`infra/api`:** an ECR repository with a lifecycle policy, `package_type =
   "Image"`, `image_uri`, and the execution role's pull permissions.
3. **The placeholder ordering inverts.** Today the layer is applied with a
   placeholder zip and the artefact follows. `image_uri` must resolve at apply
   time, so an image has to exist in ECR before `terraform apply` — the
   equivalent of `infra/api/placeholder/bootstrap` has to be re-invented for
   images, or the ordering documented as a first-create exception.
4. **The image itself**, multi-stage: a Rust build stage, a runtime stage
   carrying `/opt/extensions/lambda-adapter` and the server binary, with
   `AWS_LWA_PORT` and `AWS_LWA_READINESS_CHECK_PATH` set as they are today.
   `AWS_LAMBDA_EXEC_WRAPPER` belongs to the zip form and is expected to go away —
   confirm against the adapter's documentation before relying on it.
5. **The deploy path splits across the devcontainer boundary.** Build, ECR
   login, push, and `update-function-code --image-uri` run as a sequence
   invoked on the host, not as a devcontainer `just` recipe — resolved in step 1
   above. Whether any part of this is still worth expressing as a `just` recipe
   the host also has `just` to run, or as a plain script, is a detail for when
   this step is implemented, not a design decision this log needs to settle
   first.
6. **Documents:** a Decision Record for the packaging decision; the glibc
   headroom constraint in `docs/design/deployment.md` rewritten, since it stops
   being true; `workspace.md` only if the devcontainer changes after all.

### 2026-08-15 — steps 2 through 6 carried out

**`infra/api/Dockerfile`.** Two stages, both `public.ecr.aws/lambda/provided:al2023`
— the build stage installs a C toolchain via `microdnf` and Rust via `rustup`,
reading the version from the repository's `rust-toolchain.toml` rather than
naming it a second time; the runtime stage copies in the Lambda Web Adapter
from `public.ecr.aws/awsguru/aws-lambda-adapter:1.0.1` as an extension and the
built binary as `/var/task/bootstrap`, with that binary as `ENTRYPOINT`
directly. `AWS_LAMBDA_EXEC_WRAPPER` is gone, matching the plan's expectation —
nothing in the adapter's documented container-image usage calls for it, since
there is no layer left to redirect the entry point to. A root `.dockerignore`
keeps `target/`, `.git/`, and every `infra/**/.terraform`, `.tfstate` and
`.tfvars` out of the build context, the last group because the context would
otherwise carry credentials-adjacent state into an image layer.

**`infra/api/ecr.tf`** (new file). An `aws_ecr_repository`, a lifecycle policy
expiring untagged images after a day — the only thing worth expiring, since
`deploy-api` always pushes the same `latest` tag and never accumulates a
second tagged image — and an `aws_ecr_repository_policy` granting
`lambda.amazonaws.com` pull access. That policy's `aws:SourceArn` condition is
built from `local.name` and `data.aws_caller_identity.current`, not read from
`aws_lambda_function.api.arn`, specifically to avoid a cycle: the function
cannot be created without an image already in the repository, so a policy that
depended on the function first could never be satisfied on a first create.

**`infra/api/lambda.tf`.** `package_type = "Image"`, `image_uri` pointing at
the repository's `latest` tag, `ignore_changes = [image_uri]` in place of the
zip form's `ignore_changes = [filename, source_code_hash]`. `runtime`,
`handler`, `layers` and the `archive_file` placeholder data source are gone;
`infra/api/placeholder/` is deleted along with the `hashicorp/archive`
provider requirement, since a container image has no equivalent of a
bytes-free stub — see the Plan's step 3, resolved by documenting the ordering
in `deployment.md`'s constraints rather than by inventing one.

**`infra/api/variables.tf`.** `lambda_web_adapter_layer_arn` is gone; nothing
reads a layer ARN anymore. `lambda_architecture`'s description now points at
the Dockerfile and the adapter image's arch tag instead of a layer ARN.

**`infra/api/outputs.tf` and `ssm.tf`.** `ecr_repository_url` published
alongside `lambda_function_name`, for the host-side `deploy-api` to resolve
where to build and push.

**`justfile`'s `deploy-api`.** Rewritten around `docker build` /
`docker push` / `update-function-code --image-uri`, with a comment banner
recording that it runs on the host, not the devcontainer, and why — resolved
in step 1.

**`docs/design/deployment.md`.** The running-system table row, the `infra/`
tree listing, "The API's runtime shape", the "API — `just deploy-api`"
paragraph, the SSM parameter table, and five Constraints bullets (the glibc
headroom bullet, the `zip`-dependency bullet, the `ignore_changes` bullet, and
the `x86_64`/arm64 bullet) are rewritten for the container-image form. A third
top-of-document note records that the migration is drafted but not applied —
the deployed function is still the zip-and-layer form until the steps below
run. This edit is a draft; per `docs/README.md`'s ownership rule it needs
confirmation before the work here is complete.

**`docs/decisions/DR-0026-the-api-is-packaged-as-a-container-image.md`**
(new). Records the decision this log already made, the two alternatives
weighed and the trap-as-alternative rejected outright, and the consequences —
most namely that migrating an *already-running* function replaces it, since
`package_type` cannot change in place, which is the one apply in this
document that is not the zero-downtime kind.

**What was not done, and cannot be done from here.** This environment has no
container engine, so `infra/api/Dockerfile` has never actually been built —
every claim about what `microdnf` and `rustup` install successfully inside
`public.ecr.aws/lambda/provided:al2023` is unverified. Building it is the
first real test of this decision, and it runs on the host per the resolution
above. No `terraform apply` has run either; the two-step apply sequence this
log's Constraints addition to `deployment.md` describes — ECR repository
first, image pushed, then the function — is written but not exercised. The
currently deployed function is untouched and still answers requests the same
way it did when this log's Findings section left off.

### 2026-08-15 — engine access into the devcontainer, revisited

Step 1's resolution — build on the host, keep the devcontainer engine-less
until GitHub Actions lands — is reopened, on different grounds than the ones
weighed when it was written. The choice this time is not between the host and
a CI runner; it is between two ways of giving the devcontainer engine access at
all, weighed against standing up GitHub Actions as CD instead.

**Docker-in-Docker was considered and rejected.** A nested engine inside the
devcontainer carries its own storage driver, its own daemon, and the usual
overlay-filesystem overhead of running a container engine inside a container —
weight with no return here, since nothing needs an isolated engine; it only
needs to reach the one the host already runs.

**Docker-outside-of-Docker is the direction — applied to podman.** The host
runs podman, not Docker (`container=podman` in the environment, found while
resolving step 1 the first time). The devcontainer gains a client — `docker`
or a podman-compatible equivalent — and the host's engine socket is mounted
in, so `docker login` / `docker build -f infra/api/Dockerfile` / `docker push`
in `deploy-api` (`justfile:381-383`) reach a real engine without a second one
running inside the devcontainer. Those three commands are unchanged either
way; only whether the socket resolves changes.

**GitHub Actions CD was weighed as the alternative and judged heavier, for
now.** It needs an OIDC trust relationship between GitHub and AWS — new
Terraform, arguably a sixth root module or an extension of `api` — and it has
to encode the one ordering constraint `docs/design/deployment.md`'s
Constraints section already calls out as unsafe rather than merely
inconvenient: `just tf-apply api` before `deploy-api`, misattributing every
request if reversed. Getting that wrong in a workflow is a mistake against the
one production environment this project has. The socket-mount change is
local, reversible by rebuilding the devcontainer, and touches no AWS trust
relationship — it goes first.

**Done, 2026-08-15:** the container-engine-client feature (`docker-ce-cli`,
hand-rolled — see below) and the socket mount are both in
`.devcontainer/devcontainer.json` and confirmed working: `docker version`
reaches `Server: Podman Engine 6.1.0` from inside a live devcontainer session,
with no `sudo` needed — the UID-1000 match holds.

**Still open:**

- The push-and-update leg of `deploy-api` (`docker login`, `push`,
  `update-function-code`, `wait function-updated`) exercised from inside the
  devcontainer specifically. The migration has been deployed and confirmed
  live — see the main Verification section — but only the `docker build` leg
  was observed running from inside the devcontainer this session; which side
  ran the rest of the recipe was not.
- `docs/design/deployment.md`'s "This recipe runs on the host, not in the
  devcontainer" note, and the constraint bullet naming `docker` as a host
  dependency, still need rewriting once the push-and-update leg is confirmed
  too — not before, since the note is accurate until then.
- Whether this earns its own Decision Record, or is folded into whatever
  record eventually closes this log, is a `work-done` question once it is
  built and checked; DR-0026 already states the host-build arrangement and the
  reasoning for it, so a record here would need to say what changed and why,
  not repeat DR-0026's Context.

### 2026-08-15 — the devcontainer/Dockerfile change, made

`docker-ce-cli` (client only, no daemon, no `containerd.io`) is installed in
`.devcontainer/Dockerfile` from Docker's own apt repository, added ahead of
the `vscode` user's creation — not from the `docker-outside-of-docker`
devcontainer feature, on instruction. `devcontainer.json` gained a `mounts`
entry bind-mounting the host's rootless podman socket,
`/run/user/1000/podman/podman.sock`, to `/var/run/docker.sock` — the `docker`
CLI's own default socket path, so no `DOCKER_HOST` is set. No group or GID
matching was added: the container's `vscode` user is UID 1000, stated to
already match the host user, so a rootless podman socket owned by that UID is
directly readable and writable without one.

**The workspace mount was also changed, on instruction, though the Plan above
judged it unnecessary for `deploy-api` specifically.** `workspaceMount` and
`workspaceFolder` now both resolve to `${localWorkspaceFolder}`, so the
container's path for the repository is identical to the host's, rather than
the devcontainer default of `/workspaces/<folder-name>`. Reasoning: `docker
build`'s context is uploaded as a tar regardless of where either side mounts
the repository, so this was not required for `deploy-api` as it stands —
but it removes a latent trap for anything added later that passes a host path
through the socket, such as a bind-mounted `docker run -v`. The Dockerfile's
own `ARG WORKSPACE_DIR=/workspace` / `WORKDIR ${WORKSPACE_DIR}` is now dead at
runtime — VS Code cds into `workspaceFolder` regardless of the image's
`WORKDIR` — and was left as is, since it was already superseded before this
change: the live session's path was already `/workspaces/rust-leptos-axum-aws`
before today, not `/workspace`, so nothing that worked before this depended on
it.

**Verified 2026-08-15, from a rebuilt devcontainer with a working session.**
The rebuild, the socket path, and the client all check out:

- The devcontainer rebuilt cleanly with the new `Dockerfile` layer —
  `docker-ce-cli` is present and working, so the apt repository add and key
  fetch succeeded.
- `/run/user/1000/podman/podman.sock` is in fact where the host's rootless
  podman socket lives — `docker version` would not reach a `Server:` block
  otherwise.
- `docker version` inside the devcontainer reaches the daemon (`Server:` side
  populated: `Podman Engine 6.1.0`), not just `Client:`.
- `docker build -f infra/api/Dockerfile .` succeeds from inside the
  devcontainer. The full deploy (`tf-apply` twice, `deploy-api`) has run and
  is confirmed live — see the Verification section's 2026-08-15 entry — but
  which side ran `deploy-api` itself (`docker login`, `push`,
  `update-function-code`) was not observed directly this session, only the
  build leg was exercised from inside the devcontainer.

**Still open:**

- `docker login` / `docker push` / `aws lambda update-function-code`, the
  rest of `deploy-api`, exercised from inside the devcontainer specifically —
  the build leg is confirmed; the push-and-update leg is not yet directly
  observed running from here.
- Whether the workspace-path change causes any regression in `trunk serve`,
  bind mounts, or anything else keyed to the container's path — nothing found
  by search depended on the old `/workspaces/rust-leptos-axum-aws` path
  string, but that search covered only `.json`, `.toml`, `Dockerfile` and
  `justfile`, and nothing since has exercised `trunk serve` specifically to
  confirm no regression.

`docs/design/deployment.md`'s host-only note for `deploy-api` stays as it is
until the push-and-update leg is confirmed too — the build leg alone is not
the whole recipe.

## Verification

What could be checked from inside this environment was checked; what needs
the host's container engine or real AWS credentials could not be, and is
listed separately below.

| Checked | Result |
| --- | --- |
| `just tf-validate` (all five layers, including `api`) | passes |
| `terraform fmt -recursive -check infra` | passes |
| `just --list` | parses; `deploy-api` and every other recipe still list |
| `infra/api/.terraform.lock.hcl` | the `hashicorp/archive` entry dropped, `hashicorp/aws` left at its prior pinned version — a minimal diff, not a provider upgrade |

Earlier, before this session's work, the deployed system's own state was
checked and is unchanged by anything above:

| Checked | Result |
| --- | --- |
| `GET /health` on the API endpoint | 500 |
| `GET /api/action-types` without a token | 401 from the authorizer, which is correct and never reaches the function |
| CloudFront root | 200; the SPA is served correctly |
| Lambda configuration | `provided.al2023`, `x86_64`, adapter layer `LambdaAdapterLayerX86:28`, `AWS_LWA_PORT=3000`, `AWS_LWA_READINESS_CHECK_PATH=/health`, `TABLE_NAME` set, `State: Active` |

### 2026-08-15 — deployed and verified against real AWS

The migration ran: `just tf-apply api` twice (ECR repository first, then the
`package_type` switch) and one `just deploy-api`. Confirmed directly, from
inside the devcontainer, with `aws login` reauthenticated:

| Checked | Result |
| --- | --- |
| `docker version` from inside the devcontainer | `Server:` side populated (podman 6.1.0) through the mounted socket — the socket-mount change works |
| `docker build -f infra/api/Dockerfile .` from inside the devcontainer | succeeds |
| `objdump -T` on the extracted `/var/task/bootstrap`, highest `GLIBC_` symbol required | `2.34` — exactly what `public.ecr.aws/lambda/provided:al2023`'s own `ldd --version` reports, confirming the headroom DR-0026 claims is structural, not assumed |
| `just tf-plan api` | `No changes. Your infrastructure matches the configuration.` — the two-step apply sequence is fully landed, no drift |
| `aws_lambda_function.api` state | `package_type = "Image"`, `image_uri` resolved, `code_sha256` matches the image pushed to ECR at the same timestamp |
| `GET /health` on the API endpoint | **`200 ok`** — the outage this log exists to fix is over |
| `GET /api/action-types` without a token | `401`, from the service or the authorizer (both are live; see `docs/work/2026-08-15-service-owns-token-verification.md` for which) |

This also closes the engine-access verification opened in "the
devcontainer/Dockerfile change, made" below: the socket mount, the
`docker-ce-cli` install, and the podman socket path are all confirmed working
by the build succeeding from inside the devcontainer, not inferred.

**Not yet run** — the one check this log still owes, and the reason it stays
open:

- `GET /api/action-types` with a real, valid token reaching the function and
  being attributed to the token's subject. Obtaining that token needs an
  interactive sign-in through the Cognito hosted UI (`just dev-web-auth`,
  DR-0010) — nothing this environment can drive without a browser and a real
  user. This is the same outstanding check
  `docs/work/2026-08-11-local-token-verification.md`'s Verification section
  lists for its own four items; both logs are unblocked by the same action.

## Retirement

- [x] Design Documents updated — `docs/design/deployment.md`, drafted and
      updated again 2026-08-15 to record the migration as applied and
      verified; **awaiting the confirmation `docs/README.md` requires before
      an overwrite of a Design Document counts as done**
- [x] Decision Records written — DR-0026
- [x] Non-obvious knowledge preserved — the two symbol families' different
      origins and why both are fatal, and cargo fingerprints not seeing a libc
      change, are DR-0026's Context; that a container image only helps when the
      build is inside it is DR-0026's Alternatives, naming it as a trap rather
      than a real option; the ECR repository policy's circular-dependency
      avoidance is `infra/api/ecr.tf`'s own comment, beside the code it explains
- [ ] No durable document depends on this log — not yet true:
      `deployment.md`'s top-of-document note still cites this log by name, for
      the one check it still owes (a real token reaching `/api/action-types`)

**This log stays open for one reason only.** The migration is applied,
`GET /health` answers `ok`, and `terraform plan` on `infra/api` shows no
drift — the outage this log exists to fix is over. What remains is a single
check that needs an interactive Cognito sign-in this environment cannot
perform on its own: a real token reaching `/api/action-types` and being
attributed to its subject. That closes this log when it runs.
