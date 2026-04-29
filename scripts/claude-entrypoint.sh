#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/agent-entrypoint-lib.sh
source "$SCRIPT_DIR/agent-entrypoint-lib.sh"

KEY_FILE="${ANTHROPIC_API_KEY_FILE:-/run/secrets/anthropic_api_key}"
GITHUB_TOKEN_FILE="${GITHUB_TOKEN_FILE:-/run/secrets/github_token}"

load_secret_from_file ANTHROPIC_API_KEY "$KEY_FILE"
load_secret_from_file GITHUB_TOKEN "$GITHUB_TOKEN_FILE"

# Keep Claude session file inside persisted .claude dir and expose legacy path.
if [[ "$(id -u)" -eq 0 ]]; then
  mkdir -p /home/devops/.claude
  touch /home/devops/.claude/.claude.json
  ln -sfn /home/devops/.claude/.claude.json /home/devops/.claude.json
  chown -R devops:devops /home/devops/.claude
  chown -h devops:devops /home/devops/.claude.json || true
fi

if [[ "$#" -eq 0 ]]; then
  set -- claude
elif [[ "${1:0:1}" == "-" ]]; then
  set -- claude "$@"
fi

exec_as_devops_or_current "$@"
