#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

set -a
source "$ROOT_DIR/.env"
set +a

docker build \
  -f "$ROOT_DIR/Dockerfile.claude" \
  --build-arg CLAUDE_NODE_IMAGE="$CLAUDE_NODE_IMAGE" \
  --build-arg CLAUDE_CODE_VERSION="$CLAUDE_CODE_VERSION" \
  -t "$CLAUDE_IMAGE_NAME" \
  "$ROOT_DIR"
