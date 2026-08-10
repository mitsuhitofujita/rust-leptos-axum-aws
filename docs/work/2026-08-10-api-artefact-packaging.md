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

1. **Decide where images are built.** The devcontainer has no engine. Either the
   host's podman socket is mounted and a client installed, or the build runs on
   the host, or it moves to CodeBuild. This choice comes first because the deploy
   recipe's shape follows from it.
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
5. **`just deploy-api`** becomes build, ECR login, push, and
   `update-function-code --image-uri`.
6. **Documents:** a Decision Record for the packaging decision; the glibc
   headroom constraint in `docs/design/deployment.md` rewritten, since it stops
   being true; `workspace.md` only if the devcontainer changes after all.

## Verification

Nothing is fixed. As of this entry:

| Checked | Result |
| --- | --- |
| `GET /health` on the API endpoint | 500 |
| `GET /api/action-types` without a token | 401 from the authorizer, which is correct and never reaches the function |
| CloudFront root | 200; the SPA is served correctly |
| Lambda configuration | `provided.al2023`, `x86_64`, adapter layer `LambdaAdapterLayerX86:28`, `AWS_LWA_PORT=3000`, `AWS_LWA_READINESS_CHECK_PATH=/health`, `TABLE_NAME` set, `State: Active` |

The function is healthy by every measure Lambda reports; only its packaged
executable is unloadable.

## Retirement

- [ ] Design Documents updated — `deployment.md` at minimum, whose glibc
      constraint this work invalidates
- [ ] Decision Records written (DR-____) — the packaging decision above
- [ ] Non-obvious knowledge preserved — that the two symbol families have
      different origins and both are fatal; that cargo fingerprints do not see a
      libc change; that a container image only helps when the build is inside it
- [ ] No durable document depends on this log
