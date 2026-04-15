#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

set -a
source "$ROOT_DIR/.env"
set +a

docker build \
  --build-arg NODE_IMAGE="$NODE_IMAGE" \
  --build-arg CLAUDE_CODE_VERSION="$CLAUDE_CODE_VERSION" \
  -t "$IMAGE_NAME" \
  "$ROOT_DIR"
