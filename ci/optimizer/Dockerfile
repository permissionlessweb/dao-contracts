# Custom CosmWasm optimizer for workspaces with local sibling dependencies.
#
# The standard cosmwasm/optimizer mounts only the workspace directory at /code,
# but our [patch.crates-io] references sibling directories (../cosmwasm, ../cw-plus, etc.)
# that must also be available inside the container.
#
# This image mounts the entire parent directory tree at /workspace and runs
# the optimizer against /workspace/dao-contracts.
#
# Build:
#   docker build -t dao-optimizer:0.17.0 ci/optimizer/
#
# Run (from dao-contracts root):
#   docker run --rm -v "$(pwd)/..":/workspace \
#     --mount type=volume,source="dao_contracts_cache",target=/target \
#     --mount type=volume,source=registry_cache,target=/usr/local/cargo/registry \
#     dao-optimizer:0.17.0

FROM cosmwasm/optimizer-arm64:0.17.0

# Upgrade Rust toolchain — some deps (darling 0.23, serde_with 3.18) require rustc >= 1.88
RUN rustup install 1.88.0 && rustup default 1.88.0
# Add the wasm target for the new toolchain
RUN rustup target add wasm32-unknown-unknown --toolchain 1.88.0

# Disable bulk memory operations for CosmWasm MVP compatibility
ENV RUSTFLAGS="-C target-feature=-bulk-memory"

# Override the entrypoint with our wrapper that handles the /workspace layout
COPY dao-optimize.sh /usr/local/bin/dao-optimize.sh
RUN chmod +x /usr/local/bin/dao-optimize.sh

# Default workspace subdirectory (relative to /workspace)
ENV DAO_WORKSPACE_SUBDIR=dao-contracts

ENTRYPOINT ["dao-optimize.sh"]
CMD ["."]