# The SSM parameter root, and the prefix every resource name derives from. This
# mirrors `variable "project"` in infra/*/variables.tf; Terraform and this file
# are kept in step by hand, because a deploy reads SSM and never opens state.
project := "rust-leptos-axum-aws"

default:
    @just --list

# Frontend dev server on http://localhost:8080 (proxies /api to the API server).
dev-web:
    trunk serve

# API dev server on http://localhost:3000.
dev-api:
    cargo run -p server

# Build everything the way it would be shipped.
build:
    cargo build --workspace --release
    trunk build --release

check:
    cargo check --workspace
    cargo check -p app --target wasm32-unknown-unknown

lint:
    cargo clippy --workspace --all-targets -- -D warnings
    cargo clippy -p app --target wasm32-unknown-unknown -- -D warnings

fmt:
    cargo fmt --all

fmt-check:
    cargo fmt --all --check

test:
    cargo test --workspace

clean:
    cargo clean
    rm -rf dist

# --- Infrastructure -------------------------------------------------------
#
# Four independent root modules, one state file each (DR-0005). Terraform sees
# no relationship between them, so the order is maintained here and in
# docs/design/deployment.md and nowhere else:
#
#   create:  bootstrap -> delivery -> identity -> api
#   destroy: api -> identity -> delivery -> bootstrap
#
# bootstrap is the exception to tf-init: its first init is local, because it is
# what creates the bucket the others store state in. See
# infra/bootstrap/backend.tf.example.

# Point a layer at the remote state bucket. LAYER is delivery, identity or api.
tf-init LAYER:
    terraform -chdir=infra/{{LAYER}} init -backend-config=../backend.hcl

tf-fmt:
    terraform fmt -recursive infra

tf-fmt-check:
    terraform fmt -recursive -check infra

# Schema-checks every layer. Needs the provider registry but no AWS credentials
# and no backend, so it runs against an unapplied tree.
tf-validate:
    #!/usr/bin/env bash
    set -euo pipefail
    for layer in bootstrap delivery identity api; do
        echo "== ${layer}"
        terraform -chdir="infra/${layer}" init -backend=false -input=false >/dev/null
        terraform -chdir="infra/${layer}" validate
    done

tf-plan LAYER:
    terraform -chdir=infra/{{LAYER}} plan

tf-apply LAYER:
    terraform -chdir=infra/{{LAYER}} apply

# --- Deploying artefacts ---------------------------------------------------
#
# Terraform owns the bucket and the function; it does not own their contents
# (DR-0005). These recipes are the other half. They resolve every name from SSM
# rather than from Terraform state, so a deploy needs no backend.hcl and no
# `terraform init` — see "What each layer publishes" in
# docs/design/deployment.md.
#
# The two deploys are independent and have no ordering between them (DR-0001);
# there is deliberately no recipe that runs both.

# Read one of this project's published SSM parameters.
_ssm PATH:
    @aws ssm get-parameter --name "/{{project}}/{{PATH}}" --query Parameter.Value --output text

# Build the SPA and publish it to S3, then invalidate the CDN.
deploy-web:
    #!/usr/bin/env bash
    set -euo pipefail

    # crates/app/src/api.rs reads API_BASE_URL at compile time, so the endpoint
    # is resolved here and handed to the build. The trailing slash API Gateway
    # publishes on the $default stage's invoke URL is dropped, which the crate
    # also guards against. The app client id and the hosted-UI domain are not
    # passed yet: nothing in crates/ reads them.
    api_endpoint="$(just _ssm api/api_endpoint)"
    API_BASE_URL="${api_endpoint%/}" trunk build --release

    bucket="$(just _ssm delivery/spa_bucket)"
    distribution="$(just _ssm delivery/cloudfront_distribution_id)"
    immutable="public, max-age=31536000, immutable"

    # Order matters: every hashed asset goes up before index.html, so the entry
    # point never references a file that is not there yet. Each pass carries
    # --delete, which drops the previous build's files in the same sweep;
    # excluded keys are exempt from the delete as well as from the upload, so
    # the passes do not clobber one another.

    # 1. Hashed assets. A new build renames them, so they can be cached forever.
    aws s3 sync dist/ "s3://${bucket}/" --delete \
        --exclude index.html --exclude 'public/*' --exclude '*.wasm' \
        --cache-control "${immutable}"

    # 2. The wasm bundle, hashed like the rest but needing an explicit type: the
    #    CLI guesses from the extension, and without application/wasm the
    #    wasm-bindgen glue falls back from instantiateStreaming to the slower
    #    non-streaming compile and warns in every visitor's console.
    aws s3 sync dist/ "s3://${bucket}/" --delete \
        --exclude '*' --include '*.wasm' \
        --content-type application/wasm --cache-control "${immutable}"

    # 3. public/ is copied verbatim by trunk, so these names are stable across
    #    builds and a long cache would pin a stale favicon indefinitely.
    aws s3 sync dist/public/ "s3://${bucket}/public/" --delete \
        --cache-control "public, max-age=300"

    # 4. The entry point, last and uncached. CloudFront already refuses to cache
    #    it (Managed-CachingDisabled); this is the matching instruction to the
    #    browser, which the cache policy does not give.
    aws s3 cp dist/index.html "s3://${bucket}/index.html" \
        --cache-control "no-cache"

    # Only public/* strictly needs this — hashed assets change name and
    # index.html is never cached — but invalidations are free at this volume and
    # a wildcard survives changes to the cache behaviour above.
    aws cloudfront create-invalidation \
        --distribution-id "${distribution}" --paths '/*' \
        --query 'Invalidation.Id' --output text

# Build crates/server for Lambda and publish it.
deploy-api:
    #!/usr/bin/env bash
    set -euo pipefail

    function="$(just _ssm api/lambda_function_name)"
    target=x86_64-unknown-linux-gnu

    # No cross-compiler: the function is x86_64 (var.lambda_architecture) and so
    # is the devcontainer. provided.al2023 ships glibc 2.34, which is the
    # highest version this binary's symbols require today — a dependency that
    # pulls in a newer one would fail at invocation, not at build.
    cargo build -p server --release --target "${target}"

    # provided.al2023 requires the executable to be named `bootstrap`, which is
    # unrelated to the Lambda Web Adapter's own /opt/bootstrap.
    staging=target/lambda
    rm -rf "${staging}"
    mkdir -p "${staging}"
    cp "target/${target}/release/server" "${staging}/bootstrap"
    (cd "${staging}" && zip -q bootstrap.zip bootstrap)

    aws lambda update-function-code \
        --function-name "${function}" \
        --zip-file "fileb://${staging}/bootstrap.zip" \
        --query 'LastModified' --output text

    # update-function-code returns before the new code is live.
    aws lambda wait function-updated --function-name "${function}"
