#!/usr/bin/env bash

load_secret_from_file() {
  local env_name="$1"
  local secret_file="$2"

  if [[ -z "${!env_name:-}" && -e "$secret_file" ]]; then
    if [[ -r "$secret_file" ]]; then
      local secret_value
      secret_value="$(tr -d '\r\n' < "$secret_file")"
      export "$env_name=$secret_value"
    else
      echo "Secret file exists but is not readable: $secret_file" >&2
      exit 1
    fi
  fi
}

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

configure_git_for_devops() {
  if ! command -v git >/dev/null 2>&1; then
    return
  fi

  if [[ -n "${GIT_USER_NAME:-}" ]]; then
    gosu devops git config --global user.name "$GIT_USER_NAME"
  fi
  if [[ -n "${GIT_USER_EMAIL:-}" ]]; then
    gosu devops git config --global user.email "$GIT_USER_EMAIL"
  fi
  if [[ -n "${GITHUB_TOKEN:-}" ]]; then
    gosu devops git config --global credential.helper store
    printf 'https://x-access-token:%s@github.com\n' "$GITHUB_TOKEN" | \
      gosu devops tee /home/devops/.git-credentials >/dev/null
    gosu devops chmod 600 /home/devops/.git-credentials
  fi
}

exec_as_devops_or_current() {
  if [[ "$(id -u)" -eq 0 ]]; then
    if ! command -v gosu >/dev/null 2>&1; then
      echo "gosu is required but not installed" >&2
      exit 1
    fi

    if ! command -v git >/dev/null 2>&1; then
      exec gosu devops "$@"
    fi

    configure_git_for_devops
    exec gosu devops "$@"
  fi

  configure_git_identity
  configure_github_credentials
  exec "$@"
}
