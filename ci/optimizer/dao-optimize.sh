#!/bin/ash
# shellcheck shell=dash
# Custom optimizer entrypoint for workspaces with local sibling dependencies.
#
# The standard optimizer mounts a single directory at /code, but our workspace
# uses [patch.crates-io] with relative paths like ../cosmwasm, ../cw-plus, etc.
# This script mounts the entire parent tree at /workspace and builds from the
# correct subdirectory.
#
# Expected mount: -v /path/to/abstract:/workspace
# The DAO_WORKSPACE_SUBDIR env var (default: dao-contracts) selects which
# subdirectory under /workspace is the Cargo workspace root.
set -o errexit -o nounset -o pipefail

export PATH="$PATH:/root/.cargo/bin"

# Resolve the workspace subdirectory
SUBDIR="${DAO_WORKSPACE_SUBDIR:-dao-contracts}"
WORKSPACE_ROOT="/workspace"
PROJECT_DIR="$WORKSPACE_ROOT/$SUBDIR"

if [ ! -d "$PROJECT_DIR" ]; then
  echo "ERROR: Project directory $PROJECT_DIR does not exist." >&2
  echo "Make sure to mount the parent directory tree at /workspace." >&2
  echo "Example: docker run -v /path/to/abstract:/workspace ..." >&2
  exit 1
fi

if [ ! -f "$PROJECT_DIR/Cargo.toml" ]; then
  echo "ERROR: No Cargo.toml found at $PROJECT_DIR" >&2
  exit 1
fi

# Debug info
echo "=== DAO Custom Optimizer ==="
echo "Workspace root:  $WORKSPACE_ROOT"
echo "Project dir:     $PROJECT_DIR"
rustup toolchain list
cargo --version

# Prepare artifacts directory inside the project dir
mkdir -p "$PROJECT_DIR/artifacts"

# Delete previously built artifacts from cache
rm -f /target/wasm32-unknown-unknown/release/*.wasm

# Hide excluded crates from bob's filesystem scanner.
# bob discovers crates by scanning for Cargo.toml files — it does NOT read
# workspace.exclude. Move them to a temp location and restore after build.
EXCLUDED_CRATES="dao-migrator"
STASH_DIR="/tmp/_dao_optimizer_stash"
RESTORE=0

for crate in $EXCLUDED_CRATES; do
  CRATE_PATH="$PROJECT_DIR/contracts/external/$crate"
  if [ -d "$CRATE_PATH" ]; then
    echo "Stashing excluded crate: $crate"
    mkdir -p "$STASH_DIR"
    mv "$CRATE_PATH" "$STASH_DIR/"
    RESTORE=1
  fi
done

# Build: cd into the project directory and run bob (the optimizer's builder)
echo "Building project $PROJECT_DIR ..."
(
  cd "$PROJECT_DIR"
  /usr/local/bin/bob .
)
BUILD_EXIT=$?

# Restore stashed crates
if [ "$RESTORE" -eq 1 ]; then
  echo "Restoring stashed crates ..."
  for crate in "$STASH_DIR"/*; do
    [ -d "$crate" ] && mv "$crate" "$PROJECT_DIR/contracts/external/"
  done
  rmdir "$STASH_DIR" 2>/dev/null || true
fi

if [ "$BUILD_EXIT" -ne 0 ]; then
  exit $BUILD_EXIT
fi

# Optimize: run wasm-opt on each built .wasm
echo "Optimizing artifacts ..."
for WASM in /target/wasm32-unknown-unknown/release/*.wasm; do
  [ -e "$WASM" ] || continue

  OUT_FILENAME=$(basename "$WASM")
  echo "Optimizing $OUT_FILENAME ..."
  wasm-opt -Os "$WASM" -o "$PROJECT_DIR/artifacts/$OUT_FILENAME"
done

# Post-process: checksums
echo "Post-processing artifacts..."
(
  cd "$PROJECT_DIR/artifacts"

  if test -n "$(find . -maxdepth 1 -name '*.wasm' -print -quit)"; then
    sha256sum -- *.wasm | tee checksums.txt
  else
    echo "Warn: No .wasm file built. Check your build configuration in Cargo.toml."
  fi
)

echo "Done."
