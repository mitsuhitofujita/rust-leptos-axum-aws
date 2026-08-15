FROM docker.io/amazon/aws-cli:latest AS aws-cli

FROM rust:1.96-slim-trixie

RUN apt-get update && \
    DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends \
        curl \
        # DynamoDB Local is a jar, and this is the only reason a Rust image has
        # a JVM: nothing in the workspace compiles to it and no Java is written.
        # See the pinned archive below and DR-0020.
        default-jre-headless \
        git \
        jq \
        just \
        ripgrep \
        unzip \
        zip
RUN apt-get autoremove -y && \
    apt-get clean && \
	rm -rf /var/lib/apt/lists/*

ENV AWS_CLI_HOME=/usr/local/aws-cli
COPY --from=aws-cli $AWS_CLI_HOME $AWS_CLI_HOME
COPY --from=aws-cli /usr/local/bin/ /usr/local/bin/

# terraform provisions the AWS infrastructure (DR-0004). HashiCorp publishes the
# prebuilt binary as a zip and nothing else, which is why unzip is installed
# above; keep the version pinned so the image is reproducible.
ARG TERRAFORM_VERSION=1.15.8
RUN curl -fsSL -o /tmp/terraform.zip \
        "https://releases.hashicorp.com/terraform/${TERRAFORM_VERSION}/terraform_${TERRAFORM_VERSION}_linux_amd64.zip" && \
    unzip -q /tmp/terraform.zip terraform -d /usr/local/bin && \
    chmod +x /usr/local/bin/terraform && \
    rm /tmp/terraform.zip

# DynamoDB Local is what local verification runs the DynamoDB half of
# crates/server against, so that half is executed somewhere other than AWS
# (DR-0020). AWS distributes it as a zip, like terraform above, so unzip covers
# both and the headless JRE installed with the packages above runs it.
#
# The dated archive is used rather than dynamodb_local_latest.zip so the image is
# reproducible, and the digest is the one AWS publishes beside it — a dated URL
# alone is a name, the checksum is what makes it a pin. The v2.x path segment is
# the distribution's and not the build's version; there is no v3.x.
#
# The host is AWS's own public download endpoint for DynamoDB Local, unrelated to
# the CloudFront distribution infra/delivery creates: that one's domain is never
# written down here, it is published to SSM and resolved at deploy time (DR-0005).
# Nothing about this account is in this URL, and the checksum above is what makes
# trusting the URL unnecessary in the first place.
ARG DYNAMODB_LOCAL_VERSION=2026-07-31
ARG DYNAMODB_LOCAL_SHA256=5b0d17dd3b4e929db64a9f624a3f96eaf0961e3cf4acece00091656aec5fc7ed
RUN curl -fsSL -o /tmp/dynamodb-local.zip \
        "https://d1ni2b6xgvw0s0.cloudfront.net/v2.x/dynamodb_local_${DYNAMODB_LOCAL_VERSION}.zip" && \
    echo "${DYNAMODB_LOCAL_SHA256}  /tmp/dynamodb-local.zip" | sha256sum -c - && \
    unzip -q /tmp/dynamodb-local.zip -d /opt/dynamodb-local && \
    rm /tmp/dynamodb-local.zip

# The devcontainer reaches the host's own container engine over its socket
# (Docker-outside-of-Docker, applied to podman — the host's actual engine)
# rather than running a nested one, so only a client is needed here; the
# socket itself is bind-mounted in devcontainer.json's `mounts`.
RUN install -m 0755 -d /etc/apt/keyrings && \
    curl -fsSL https://download.docker.com/linux/debian/gpg -o /etc/apt/keyrings/docker.asc && \
    chmod a+r /etc/apt/keyrings/docker.asc && \
    echo "deb [arch=$(dpkg --print-architecture) signed-by=/etc/apt/keyrings/docker.asc] https://download.docker.com/linux/debian $(. /etc/os-release && echo "$VERSION_CODENAME") stable" \
        > /etc/apt/sources.list.d/docker.list && \
    apt-get update && \
    DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends docker-ce-cli && \
    apt-get clean && \
    rm -rf /var/lib/apt/lists/*

ARG VSCODE_UID=1000
ARG VSCODE_USER=vscode
ARG VSCODE_GID=1000
ARG VSCODE_GROUP=vscode
ARG VSCODE_HOME=/home/vscode
ARG WORKSPACE_DIR=/workspace
RUN groupadd -g ${VSCODE_GID} ${VSCODE_GROUP} && \
    useradd -g ${VSCODE_GROUP} -u ${VSCODE_UID} -d ${VSCODE_HOME} -m ${VSCODE_USER} && \
    mkdir -p ${WORKSPACE_DIR}

WORKDIR ${WORKSPACE_DIR}

USER ${VSCODE_USER}

RUN rustup component add clippy rustfmt && \
    rustup target add wasm32-unknown-unknown

# trunk builds the CSR WASM frontend. The prebuilt binary is used because
# building it from source takes minutes; keep the version pinned so the image is
# reproducible. wasm-bindgen is not installed here: trunk downloads a CLI
# matching the project's wasm-bindgen version into its own cache at build time.
ARG TRUNK_VERSION=0.21.14
RUN curl -fsSL "https://github.com/trunk-rs/trunk/releases/download/v${TRUNK_VERSION}/trunk-x86_64-unknown-linux-gnu.tar.gz" \
        | tar -xzC "${CARGO_HOME}/bin" trunk && \
    chmod +x "${CARGO_HOME}/bin/trunk"

RUN curl -fsSL https://claude.ai/install.sh | bash
