#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

set -a
source "$ROOT_DIR/.env"
set +a

docker build \
  -f "$ROOT_DIR/Dockerfile.codex" \
  --build-arg CODEX_NODE_IMAGE="$CODEX_NODE_IMAGE" \
  --build-arg CODEX_VERSION="$CODEX_VERSION" \
  -t "$CODEX_IMAGE_NAME" \
  "$ROOT_DIR"
