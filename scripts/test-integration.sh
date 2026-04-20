#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
AGENT_KIND="${1:-claude}"

case "$AGENT_KIND" in
  claude)
    COMPOSE_FILES=(
      -f "$ROOT_DIR/docker-compose.yml"
      -f "$ROOT_DIR/docker-compose.claude.yml"
      -f "$ROOT_DIR/.ci/docker-compose.test.yml"
    )
    AGENT_SERVICE="claude-agent"
    INSPECT_CONTAINER="ai-sandbox-claude-agent-inspect"
    SECRET_CONTAINER_FILE="/run/secrets/anthropic_api_key"
    SECRET_ENV_NAME="ANTHROPIC_API_KEY"
    SECONDARY_SECRET_CONTAINER_FILE="/run/secrets/github_token"
    SECONDARY_SECRET_ENV_NAME="GITHUB_TOKEN"
    ;;
  codex)
    COMPOSE_FILES=(
      -f "$ROOT_DIR/docker-compose.yml"
      -f "$ROOT_DIR/docker-compose.codex.yml"
      -f "$ROOT_DIR/.ci/docker-compose.test.yml"
    )
    AGENT_SERVICE="codex-agent"
    INSPECT_CONTAINER="ai-sandbox-codex-agent-inspect"
    SECRET_CONTAINER_FILE="/run/secrets/openai_api_key"
    SECRET_ENV_NAME="OPENAI_API_KEY"
    SECONDARY_SECRET_CONTAINER_FILE="/run/secrets/github_token"
    SECONDARY_SECRET_ENV_NAME="GITHUB_TOKEN"
    ;;
  *)
    echo "ERROR: unknown agent kind '$AGENT_KIND' (expected: claude|codex)" >&2
    exit 1
    ;;
esac

compose() {
  docker compose "${COMPOSE_FILES[@]}" "$@"
}

fail() {
  echo "ERROR: $1" >&2
  exit 1
}

cleanup() {
  docker rm -f "$INSPECT_CONTAINER" >/dev/null 2>&1 || true
  compose down -v --remove-orphans >/dev/null 2>&1 || true
}

trap cleanup EXIT

if [[ -z "${!SECRET_ENV_NAME:-}" ]]; then
  export "$SECRET_ENV_NAME=dummy-key-for-ci"
fi
if [[ -z "${!SECONDARY_SECRET_ENV_NAME:-}" ]]; then
  export "$SECONDARY_SECRET_ENV_NAME=dummy-github-token-for-ci"
fi

compose up -d --build proxy mock-upstream blocked-upstream tester

allowed_ready=0
for _ in $(seq 1 30); do
  if compose exec -T tester sh -lc "curl -fsS -o /dev/null -x http://proxy:3128 http://mock-upstream/"; then
    allowed_ready=1
    break
  fi
  sleep 1
done

if [[ "$allowed_ready" -ne 1 ]]; then
  fail "proxy did not become ready in time"
fi

allowed_response="$(compose exec -T tester sh -lc "curl -fsS -x http://proxy:3128 http://mock-upstream/")"
if [[ "$allowed_response" != *"ok-allowed"* ]]; then
  fail "allowed domain request did not return expected body"
fi

if compose exec -T tester sh -lc "curl -fsS -o /dev/null -x http://proxy:3128 http://blocked-upstream/"; then
  fail "blocked domain unexpectedly succeeded"
fi

mock_ip="$(compose exec -T tester sh -lc "getent hosts mock-upstream | awk 'NR==1 {print \$1}'")"
if [[ -z "$mock_ip" ]]; then
  fail "could not resolve mock-upstream IP for literal-IP test"
fi

if compose exec -T tester sh -lc "curl -fsS -o /dev/null -x http://proxy:3128 http://${mock_ip}/"; then
  fail "literal IP request unexpectedly succeeded"
fi

compose build "$AGENT_SERVICE"

compose run --rm "$AGENT_SERVICE" sh -lc \
  "test -f '$SECRET_CONTAINER_FILE' && test -n \"\$(printenv '$SECRET_ENV_NAME' || true)\""
compose run --rm "$AGENT_SERVICE" sh -lc \
  "test -f '$SECONDARY_SECRET_CONTAINER_FILE' && test -n \"\$(printenv '$SECONDARY_SECRET_ENV_NAME' || true)\""

agent_cid="$(compose run -d --name "$INSPECT_CONTAINER" "$AGENT_SERVICE" sleep 120)"
if docker inspect "$agent_cid" --format '{{range .Config.Env}}{{println .}}{{end}}' | grep -q "^${SECRET_ENV_NAME}="; then
  fail "$SECRET_ENV_NAME is present in docker inspect env"
fi
if docker inspect "$agent_cid" --format '{{range .Config.Env}}{{println .}}{{end}}' | grep -q "^${SECONDARY_SECRET_ENV_NAME}="; then
  fail "$SECONDARY_SECRET_ENV_NAME is present in docker inspect env"
fi
docker rm -f "$agent_cid" >/dev/null 2>&1 || true

echo "Integration tests passed for $AGENT_KIND"
