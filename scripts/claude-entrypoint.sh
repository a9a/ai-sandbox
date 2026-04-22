#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/agent-entrypoint-lib.sh
source "$SCRIPT_DIR/agent-entrypoint-lib.sh"

KEY_FILE="${ANTHROPIC_API_KEY_FILE:-/run/secrets/anthropic_api_key}"
GITHUB_TOKEN_FILE="${GITHUB_TOKEN_FILE:-/run/secrets/github_token}"

load_secret_from_file ANTHROPIC_API_KEY "$KEY_FILE"
load_secret_from_file GITHUB_TOKEN "$GITHUB_TOKEN_FILE"

if [[ "$#" -eq 0 ]]; then
  set -- claude
elif [[ "${1:0:1}" == "-" ]]; then
  set -- claude "$@"
fi

exec_as_devops_or_current "$@"
