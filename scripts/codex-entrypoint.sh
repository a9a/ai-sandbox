#!/usr/bin/env bash
set -euo pipefail

KEY_FILE="${OPENAI_API_KEY_FILE:-/run/secrets/openai_api_key}"

if [[ -z "${OPENAI_API_KEY:-}" && -e "$KEY_FILE" ]]; then
  if [[ -r "$KEY_FILE" ]]; then
    OPENAI_API_KEY="$(tr -d '\r\n' < "$KEY_FILE")"
    export OPENAI_API_KEY
  else
    echo "Secret file exists but is not readable: $KEY_FILE" >&2
    exit 1
  fi
fi

if [[ "$#" -eq 0 ]]; then
  set -- codex
elif [[ "${1:0:1}" == "-" ]]; then
  set -- codex "$@"
fi

if [[ "$(id -u)" -eq 0 ]]; then
  if ! command -v gosu >/dev/null 2>&1; then
    echo "gosu is required but not installed" >&2
    exit 1
  fi
  exec gosu devops "$@"
fi

exec "$@"
