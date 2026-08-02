FROM rust:1.96-slim-trixie

RUN apt-get update && \
    DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends \
        curl \
        git \
        jq \
        just \
        ripgrep
RUN apt-get autoremove -y && \
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
