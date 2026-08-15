# The SSM parameter root, and the prefix every resource name derives from. This
# mirrors `variable "project"` in infra/*/variables.tf; Terraform and this file
# are kept in step by hand, because a deploy reads SSM and never opens state.
project := "rust-leptos-axum-aws"

default:
    @just --list

# Regenerate the action-type icon catalog from the pinned lucide-leptos.
#
# Nothing runs this automatically. The generated tables and the pin agree only
# because this was run after the pin, or the category list in
# crates/app/Cargo.toml, last moved — see crates/icongen and DR-0014.
icons:
    cargo run -p icongen
    cargo fmt -p shared -p app

# Where `trunk serve` sends /api. Trunk.toml holds no [[proxy]] block, because
# there are two things it could point at and trunk appends a CLI backend to the
# file's entries rather than overriding them — see DR-0023. So each recipe below
# names the one it wants: the service directly, or the token adapter.
api_backend := "http://127.0.0.1:3000/api"
gateway_backend := "http://127.0.0.1:3001/api"

# Frontend dev server on http://localhost:8080 (proxies /api to the API server).
dev-web:
    trunk serve --proxy-backend {{api_backend}}

# Unset COGNITO_* variables mean no sign-in control and no Authorization header
# (DR-0008), which is what `dev-web` above gets and what most development wants:
# the local axum server checks nothing. This recipe is for working on the flow
# itself. The app client already lists http://localhost:8080/ among its callback
# and logout URLs, so no infrastructure change is needed to sign in locally.

# The same dev server with sign-in switched on. Needs AWS credentials.
dev-web-auth:
    #!/usr/bin/env bash
    set -euo pipefail

    COGNITO_CLIENT_ID="$(just _ssm identity/app_client_id)" \
    COGNITO_HOSTED_UI_DOMAIN="$(just _ssm identity/hosted_ui_domain)" \
        trunk serve --proxy-backend {{api_backend}}

# API dev server on http://localhost:3000.
dev-api:
    cargo run -p server

# --- Local verification against DynamoDB -----------------------------------
#
# dev-api above runs the in-memory store, so the DynamoDB half of crates/server
# is compiled by every build and executed by nothing until it is deployed
# (DR-0018). The recipes below are how it is executed here instead: DynamoDB
# Local, the table, and the same binary pointed at both.
#
# crates/server knows nothing about any of this. AWS_ENDPOINT_URL_DYNAMODB is
# read by the SDK's generated config rather than by the service, and TABLE_NAME
# is still what chooses the store — exactly as on the Lambda, which is what makes
# this a verification of the deployed path rather than of a stand-in. DR-0020.

# Kept in step with infra/ by hand, like `project` at the top of this file:
# dynamo_region mirrors `variable "region"` and dynamo_table mirrors the table's
# `"${var.project}-app"` in infra/data/main.tf.
dynamo_endpoint := "http://localhost:8000"
dynamo_region := "ap-northeast-1"
dynamo_table := project + "-app"

# -sharedDb is not optional. Without it DynamoDB Local keeps a separate database
# per access key and region, so dynamo-table below and the server would create
# and query two different tables and neither would ever say so.
#
# -inMemory means the table is gone when this stops, which is the lifetime the
# in-memory store already has; dynamo-table is re-run after every restart.
#
# -disableTelemetry stops two things. Nothing is reported to AWS by a
# verification step that exists to need no AWS at all; and without it DynamoDB
# Local writes dynamodb-local-metadata.json — an installation id and a flag —
# into the working directory, which is this repository. That file is in
# .gitignore as well, because running the jar by hand still produces it.

# DynamoDB Local on http://localhost:8000. Runs in the foreground, like dev-api.
dynamo:
    java -Djava.library.path=/opt/dynamodb-local/DynamoDBLocal_lib \
        -jar /opt/dynamodb-local/DynamoDBLocal.jar \
        -inMemory -sharedDb -disableTelemetry -port 8000

# Ctrl-C in the terminal running `just dynamo` is the ordinary way to stop it.
# This is for the other cases: it was started in a terminal that has since gone,
# or it is holding port 8000 and it is not obvious what is.
#
# There is no shutdown endpoint to ask instead: /shutdown answers 400 on 3.3.1
# and -help lists no such option, so a signal is the only way.
#
# The process is found by reading /proc rather than with pkill, because procps is
# not in the image — pkill, pgrep, ps, fuser and lsof are all absent, and only
# coreutils and bash are relied on here. Both the command name and the jar path
# are matched: the path alone also matches this recipe's own shell, whose command
# line contains it, and killing that is how a first attempt at this went wrong.

# Stop DynamoDB Local. Does nothing, successfully, if it is not running.
dynamo-stop:
    #!/usr/bin/env bash
    set -euo pipefail

    found=""
    for d in /proc/[0-9]*; do
        read -r comm < "$d/comm" 2>/dev/null || continue
        [ "$comm" = java ] || continue
        tr '\0' '\n' < "$d/cmdline" 2>/dev/null |
            grep -qxF /opt/dynamodb-local/DynamoDBLocal.jar || continue
        kill "${d#/proc/}" && found="${d#/proc/}"
    done

    if [ -z "$found" ]; then
        echo "DynamoDB Local is not running"
        exit 0
    fi

    # The table went with it: -inMemory, so dynamo-table is re-run after the
    # next start.
    echo "stopped DynamoDB Local (pid $found)"

# The key schema is a copy of infra/data/main.tf's, and both are copies of the
# interface docs/design/persistence.md defines: pk the owner, sk the entity kind
# and its ordering, nothing else declared. Billing mode is named because the API
# requires one; DynamoDB Local bills nothing.

# Create the local table, idempotently. Needs `just dynamo` running.
dynamo-table:
    #!/usr/bin/env bash
    set -euo pipefail

    export AWS_ENDPOINT_URL_DYNAMODB="{{dynamo_endpoint}}"
    export AWS_REGION="{{dynamo_region}}"
    export AWS_ACCESS_KEY_ID=local
    export AWS_SECRET_ACCESS_KEY=local

    if aws dynamodb describe-table --table-name "{{dynamo_table}}" >/dev/null 2>&1; then
        echo "{{dynamo_table}} already exists"
        exit 0
    fi

    aws dynamodb create-table \
        --table-name "{{dynamo_table}}" \
        --attribute-definitions \
            AttributeName=pk,AttributeType=S \
            AttributeName=sk,AttributeType=S \
        --key-schema \
            AttributeName=pk,KeyType=HASH \
            AttributeName=sk,KeyType=RANGE \
        --billing-mode PAY_PER_REQUEST \
        --query 'TableDescription.TableName' --output text

# The credentials are fake on purpose. DynamoDB Local checks nothing, and a
# process that cannot authenticate anywhere cannot reach the real table by
# accident — these override a real AWS session if one happens to be configured,
# which is the point of setting them rather than leaving them out.

# The API server on the DynamoDB store instead of the in-memory one.
dev-api-dynamo:
    TABLE_NAME="{{dynamo_table}}" \
    AWS_ENDPOINT_URL_DYNAMODB="{{dynamo_endpoint}}" \
    AWS_REGION="{{dynamo_region}}" \
    AWS_ACCESS_KEY_ID=local \
    AWS_SECRET_ACCESS_KEY=local \
        cargo run -p server

# --- Local verification of a real Cognito token -----------------------------
#
# crates/devgateway answers one question: would the deployed authorizer accept
# this token? It sits in front of the unmodified service, verifies the token the
# way aws_apigatewayv2_authorizer.cognito verifies it, converts what it accepts
# into the AuthContext the service reads, and refuses the rest — DR-0022,
# DR-0024, DR-0025.
#
# It reproduces nothing else about API Gateway. The route table, the preflight
# and the 401 for an unrouted method were reproduced here once and retracted,
# because each was a second telling of a specification AWS holds — DR-0023.
#
# Three terminals when this is in use: dev-api, dev-gateway, dev-web-gateway.
# Nothing about the two-terminal default changes if it never is.

# The token is verified the way the deployed authorizer verifies it: RS256
# against the pool's published keys, then `iss`, `exp`, and the app client id —
# which a Cognito access token carries in `client_id` and an id token in `aud`.
#
# What it is for is infra/api/apigateway.tf, not the service. The two values
# below are the authorizer's `jwt_configuration`, resolved from the same SSM
# parameters the api layer reads them from, so a wrong one is visible here rather
# than as a 401 after an apply. Set DEVGATEWAY_AUDIENCE by hand to something else
# to watch a good token be refused.
#
# Every path needs a token, /health included: there is no route table here, so
# the probe is authorized like anything else and answers 401 through :3001 where
# the deployment answers ok. `Bearer alice` does not work either — two callers
# without a token is dev-api and its two AuthContext headers.
#
# A dev-web bundle sends no Authorization header at all (DR-0008), so behind this
# every /api call is a 401 — which is deployment.md's constraint about a bundle
# built without the Cognito variables, reproduced locally. Use dev-web-auth for a
# browser session with a real token.
#
# Needs AWS credentials for SSM and network for the JWKS, and neither afterwards.
# The key set is fetched once before the listener binds; a pool that has rotated
# its keys since means restarting this.

# The deployed authorizer on http://localhost:3001. Needs AWS credentials.
dev-gateway:
    #!/usr/bin/env bash
    set -euo pipefail

    DEVGATEWAY_ISSUER="$(just _ssm identity/user_pool_issuer)" \
    DEVGATEWAY_AUDIENCE="$(just _ssm identity/app_client_id)" \
        cargo run -p devgateway

# Needs `dev-gateway` running as well as `dev-api`.

# The dev server with /api going through the adapter rather than to the service.
dev-web-gateway:
    trunk serve --proxy-backend {{gateway_backend}}

# ---------------------------------------------------------------------------

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
# Five independent root modules, one state file each (DR-0005). Terraform sees
# no relationship between them, so the order is maintained here and in
# docs/design/deployment.md and nowhere else:
#
#   create:  bootstrap -> delivery -> identity -> data -> api
#   destroy: api -> data -> identity -> delivery -> bootstrap
#
# bootstrap is the exception to tf-init: its first init is local, because it is
# what creates the bucket the others store state in. See
# infra/bootstrap/backend.tf.example.

# Point a layer at the remote state bucket. LAYER is delivery, identity, data or api.
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
    for layer in bootstrap delivery identity data api; do
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

    # The SPA is configured at compile time (DR-0008), so every value it needs
    # is resolved here and handed to the build: the endpoint for
    # crates/app/src/api.rs, and the app client and hosted-UI domain for
    # crates/app/src/auth.rs. The trailing slash API Gateway publishes on the
    # $default stage's invoke URL is dropped, which the crate also guards
    # against. There is no redirect URI among these: auth.rs computes it from
    # window.location.origin, so it cannot drift from the app client's
    # registered callback URLs.
    api_endpoint="$(just _ssm api/api_endpoint)"
    API_BASE_URL="${api_endpoint%/}" \
    COGNITO_CLIENT_ID="$(just _ssm identity/app_client_id)" \
    COGNITO_HOSTED_UI_DOMAIN="$(just _ssm identity/hosted_ui_domain)" \
        trunk build --release

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
