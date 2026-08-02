# DR-0003: trunk is installed into the image as a pinned prebuilt binary, and wasm-bindgen is left to trunk

Status: accepted
Date: 2026-08-02

## Context

The CSR build (DR-0001) needs `trunk`, and trunk in turn needs a
`wasm-bindgen` CLI to generate the JavaScript bindings for the compiled WASM.
Neither was present in the devcontainer image, which shipped only Rust 1.96.1
with the host target.

Two things had to be settled: how trunk gets into the environment, and what to
do about `wasm-bindgen-cli`.

The second question is the one with a history. The `wasm-bindgen` crate and the
`wasm-bindgen-cli` binary must be the same version; when they are not, the build
fails with a version-mismatch error that reads as though the toolchain is
broken. The usual reflex is to pin the CLI explicitly, which is what makes the
problem recurring rather than what solves it.

## Decision

Install trunk in `.devcontainer/Dockerfile` by unpacking the **prebuilt release
binary**, with the version pinned in an `ARG` (0.21.14 at the time of writing),
into `${CARGO_HOME}/bin`.

Do **not** install `wasm-bindgen-cli`. trunk reads the `wasm-bindgen` version
resolved in `Cargo.lock` and downloads a matching CLI into its own cache on
first use. Leaving it alone is what keeps the two in step.

`rustup target add wasm32-unknown-unknown` is also baked into the image,
alongside the `rust-toolchain.toml` declaration, so the target is present in a
freshly built container without waiting for the first cargo invocation.

## Alternatives

**`cargo install trunk --locked`.** The obvious route, and self-contained: no
dependency on release artefacts, and it works on any host architecture.
Rejected because trunk is a large binary and building it from source adds
several minutes to every image build, for a tool whose source nobody here
modifies.

**`cargo-binstall`.** Fetches the same prebuilt artefact, but adds a tool that
would exist only to install one other tool.

**Installing trunk ad hoc outside the image.** Fastest to do once, and it is how
the tool was first obtained in the running container. Rejected as the durable
answer: it vanishes on rebuild, and nothing then records which version anyone is
using.

**Pinning `wasm-bindgen-cli` alongside the `wasm-bindgen` crate.** Rejected —
see Context. It converts an automatic invariant into one maintained by hand.

## Consequences

Image builds stay fast, and the version is stated in one place, so two
containers built from the same Dockerfile get the same trunk.

What this makes hard:

- **The image build depends on GitHub releases** being reachable, and on an
  `x86_64-unknown-linux-gnu` artefact existing. A different host architecture
  needs the URL changed or a fallback to `cargo install`.
- **The pin has to be bumped by hand.** Nothing notices that a newer trunk
  exists.
- **The first `trunk build` in a fresh container needs network access**, because
  that is when wasm-bindgen is fetched. An offline first build fails in a way
  that looks unrelated to the network.

Reversing any of this is cheap: it is a few lines of Dockerfile.

One incidental note, recorded because it costs minutes to rediscover and has no
other home: **the crates.io API rejects curl's default user agent**, answering
with an empty body. It reads exactly like a network failure. Send an explicit
`User-Agent` header, or use `cargo info` / `cargo search` instead.
