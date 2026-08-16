# DR-0027: The devcontainer reaches the host's container engine over its socket

Status: accepted — narrows DR-0026's host-only build/push claim
Date: 2026-08-16

## Context

DR-0026 packaged the API as a container image and, in doing so, decided
`just deploy-api` would build and push from the host rather than the
devcontainer: the devcontainer had no container engine, and standing up one
now — a socket mount, or AWS CodeBuild — was judged work a GitHub Actions
runner would replace once deployment moved there, so neither was built at the
time.

That trade-off was reopened on 2026-08-15, on different grounds than the ones
weighed when it was written: not host-versus-CI-runner, but which of two ways
to give the devcontainer engine access at all, weighed against standing up
GitHub Actions as CD instead of either.

## Decision

The devcontainer gains a container-engine client and reaches the host's
engine over its socket — Docker-outside-of-Docker, applied to podman since
that is the host's actual engine (`container=podman` in the environment).
`.devcontainer/Dockerfile` installs `docker-ce-cli` (client only, no daemon,
no `containerd.io`) from Docker's own apt repository, ahead of the `vscode`
user's creation, rather than through the `docker-outside-of-docker`
devcontainer feature. `devcontainer.json` bind-mounts the host's rootless
podman socket, `/run/user/1000/podman/podman.sock`, to `/var/run/docker.sock`
— the `docker` CLI's own default path, so no `DOCKER_HOST` is set. No group
or GID matching was needed: the container's `vscode` user is UID 1000,
matching the host user, so the rootless socket is directly readable and
writable.

`workspaceMount` and `workspaceFolder` were also changed to both resolve to
`${localWorkspaceFolder}`, so the container's path for the repository is
identical to the host's. `docker build`'s context is uploaded as a tar
regardless of where either side mounts the repository, so this was not
required for `deploy-api` as it stands, but it removes a latent trap for
anything added later that passes a host path through the socket, such as a
bind-mounted `docker run -v`.

**Docker-in-Docker was considered and rejected.** A nested engine inside the
devcontainer carries its own storage driver, its own daemon, and the usual
overlay-filesystem overhead of running a container engine inside a
container — weight with no return here, since nothing needs an isolated
engine; it only needs to reach the one the host already runs.

**Standing up GitHub Actions as CD now was weighed as the alternative and
judged heavier, still.** It needs an OIDC trust relationship between GitHub
and AWS — new Terraform, arguably a sixth root module or an extension of
`api` — and it has to encode the one ordering constraint
`docs/design/deployment.md`'s Constraints section already calls out as unsafe
rather than merely inconvenient: `just tf-apply api` before `deploy-api`,
misattributing every request if reversed. Getting that wrong in a workflow is
a mistake against the one production environment this project has. The
socket-mount change is local, reversible by rebuilding the devcontainer, and
touches no AWS trust relationship. GitHub Actions stays future work, not
ruled out.

## Consequences

**`just deploy-api` now runs identically from the host or the devcontainer.**
Confirmed 2026-08-16: `docker build -f infra/api/Dockerfile .`,
`docker login`, `docker push`, `aws lambda update-function-code`, and
`aws lambda wait function-updated` were all run from inside the devcontainer
against the real ECR repository and the real function, and the result was
verified against the deployed API with a real Cognito access token —
`POST /api/action-types` answered `201`, and the following
`GET /api/action-types` returned that record, both against the deployed
function and the real table. This narrows DR-0026's Decision line ("The
image is built and pushed from the host, not the devcontainer") and its
Consequences line ("`just deploy-api` no longer runs inside the
devcontainer... now runs on the host"); the rest of DR-0026 — packaging as a
container image, the multi-stage build, the adapter as an extension — is
unchanged.

**Nothing about the ordering constraint changes.** `just tf-apply api` still
has to come before `just deploy-api`, and the ECR repository still has to
hold an image before the first `package_type = "Image"` apply. Which shell
runs the commands does not affect either.

**What this would cost to reverse.** Removing the socket mount and the
client from `.devcontainer/Dockerfile` and `devcontainer.json` returns
`deploy-api` to host-only, symmetrically — nothing about the artefact or the
infrastructure depends on where the commands run.
