#!/usr/bin/env bash
set -euo pipefail

KEY_FILE="${ANTHROPIC_API_KEY_FILE:-/run/secrets/anthropic_api_key}"

if [[ -z "${ANTHROPIC_API_KEY:-}" && -f "$KEY_FILE" ]]; then
  ANTHROPIC_API_KEY="$(tr -d '\r\n' < "$KEY_FILE")"
  export ANTHROPIC_API_KEY
fi

if [[ "$#" -eq 0 ]]; then
  set -- claude
elif [[ "${1:0:1}" == "-" ]]; then
  set -- claude "$@"
fi

exec "$@"
