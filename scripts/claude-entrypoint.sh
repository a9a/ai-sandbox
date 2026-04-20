#!/usr/bin/env bash
set -euo pipefail

KEY_FILE="${ANTHROPIC_API_KEY_FILE:-/run/secrets/anthropic_api_key}"

if [[ -z "${ANTHROPIC_API_KEY:-}" && -e "$KEY_FILE" ]]; then
  if [[ -r "$KEY_FILE" ]]; then
    ANTHROPIC_API_KEY="$(tr -d '\r\n' < "$KEY_FILE")"
    export ANTHROPIC_API_KEY
  else
    echo "Secret file exists but is not readable: $KEY_FILE" >&2
    exit 1
  fi
fi

if [[ "$#" -eq 0 ]]; then
  set -- claude
elif [[ "${1:0:1}" == "-" ]]; then
  set -- claude "$@"
fi

configure_git_identity() {
  if ! command -v git >/dev/null 2>&1; then
    return
  fi

  if [[ -n "${GIT_USER_NAME:-}" ]]; then
    git config --global user.name "$GIT_USER_NAME"
  fi
  if [[ -n "${GIT_USER_EMAIL:-}" ]]; then
    git config --global user.email "$GIT_USER_EMAIL"
  fi
}

configure_github_credentials() {
  if ! command -v git >/dev/null 2>&1; then
    return
  fi
  if [[ -z "${GITHUB_TOKEN:-}" ]]; then
    return
  fi

  umask 077
  git config --global credential.helper store
  printf 'https://x-access-token:%s@github.com\n' "$GITHUB_TOKEN" > "${HOME}/.git-credentials"
  chmod 600 "${HOME}/.git-credentials"
}

if [[ "$(id -u)" -eq 0 ]]; then
  if ! command -v gosu >/dev/null 2>&1; then
    echo "gosu is required but not installed" >&2
    exit 1
  fi
  if ! command -v git >/dev/null 2>&1; then
    exec gosu devops "$@"
  fi
  if [[ -n "${GIT_USER_NAME:-}" ]]; then
    gosu devops git config --global user.name "$GIT_USER_NAME"
  fi
  if [[ -n "${GIT_USER_EMAIL:-}" ]]; then
    gosu devops git config --global user.email "$GIT_USER_EMAIL"
  fi
  if [[ -n "${GITHUB_TOKEN:-}" ]]; then
    gosu devops sh -c 'umask 077; git config --global credential.helper store; cat > "${HOME}/.git-credentials"; chmod 600 "${HOME}/.git-credentials"' \
      <<< "https://x-access-token:${GITHUB_TOKEN}@github.com"
  fi
  exec gosu devops "$@"
fi

configure_git_identity
configure_github_credentials
exec "$@"
