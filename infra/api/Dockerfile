# syntax=docker/dockerfile:1.7
#
# Built on the host (docs/work/2026-08-10-api-artefact-packaging.md), never in
# the devcontainer, which has no container engine. Build context is the
# workspace root: `docker build -f infra/api/Dockerfile .`
#
# Both stages sit on the Lambda "provided.al2023" base, so the binary's glibc
# requirement never exceeds what the function actually ships — the property
# docs/design/deployment.md tracked as unenforced headroom under the zip form
# this replaces. rustup, not a Rust base image, is what lets the build stage
# keep that base rather than borrowing a newer glibc from elsewhere.

FROM public.ecr.aws/lambda/provided:al2023 AS builder

# curl and curl-minimal both provide /usr/bin/curl and conflict on install;
# provided.al2023 ships curl-minimal already, so curl itself is left out here.
RUN microdnf install -y \
        gcc \
        gcc-c++ \
        make \
        cmake \
        perl \
        tar \
        gzip \
        findutils \
        which \
    && microdnf clean all

ENV RUSTUP_HOME=/usr/local/rustup \
    CARGO_HOME=/usr/local/cargo \
    PATH=/usr/local/cargo/bin:${PATH}

WORKDIR /workspace

# Copied in before the sources, so the version this stage builds is read from
# the one file that pins it — the same file the devcontainer reads — rather
# than a second copy of the version number kept in step by hand.
COPY rust-toolchain.toml ./
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
        | sh -s -- -y --profile minimal --default-toolchain none \
    && rustup show

COPY Cargo.toml Cargo.lock ./
COPY crates ./crates

RUN cargo build -p server --release --target x86_64-unknown-linux-gnu

FROM public.ecr.aws/lambda/provided:al2023 AS runtime

# The adapter's own executable. Unrelated to /var/task/bootstrap below despite
# both being named "bootstrap" under the zip form this replaces, where the
# adapter shipped as a Lambda layer providing /opt/bootstrap instead of this
# extension — docs/design/deployment.md. Version pinned to the same 1.0.1 the
# layer form published.
COPY --from=public.ecr.aws/awsguru/aws-lambda-adapter:1.0.1 /lambda-adapter /opt/extensions/lambda-adapter

COPY --from=builder /workspace/target/x86_64-unknown-linux-gnu/release/server /var/task/bootstrap

# AWS_LWA_PORT and AWS_LWA_READINESS_CHECK_PATH are set on the function by
# Terraform (infra/api/lambda.tf), not here — one place names them, as under the
# zip form. AWS_LAMBDA_EXEC_WRAPPER does not: it made the adapter layer's
# /opt/bootstrap the entry point under the zip form, and has no role once the
# adapter is copied in as an extension and this image's own ENTRYPOINT is the
# service.
ENTRYPOINT ["/var/task/bootstrap"]
