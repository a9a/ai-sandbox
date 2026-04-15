#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
COMPOSE_FILES=(-f "$ROOT_DIR/docker-compose.yml" -f "$ROOT_DIR/.ci/docker-compose.test.yml")

compose() {
  docker compose "${COMPOSE_FILES[@]}" "$@"
}

fail() {
  echo "ERROR: $1" >&2
  exit 1
}

cleanup() {
  docker rm -f ai-sandbox-agent-inspect >/dev/null 2>&1 || true
  compose down -v --remove-orphans >/dev/null 2>&1 || true
}

trap cleanup EXIT

mkdir -p "$ROOT_DIR/secrets"
if [[ ! -f "$ROOT_DIR/secrets/anthropic_api_key.txt" ]]; then
  printf "dummy-key-for-ci\n" > "$ROOT_DIR/secrets/anthropic_api_key.txt"
  chmod 600 "$ROOT_DIR/secrets/anthropic_api_key.txt"
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

compose build agent

compose run --rm agent sh -lc '[ -f /run/secrets/anthropic_api_key ] && [ -n "${ANTHROPIC_API_KEY:-}" ]'

agent_cid="$(compose run -d --name ai-sandbox-agent-inspect agent sleep 120)"
if docker inspect "$agent_cid" --format '{{range .Config.Env}}{{println .}}{{end}}' | grep -q '^ANTHROPIC_API_KEY='; then
  fail "ANTHROPIC_API_KEY is present in docker inspect env"
fi
docker rm -f "$agent_cid" >/dev/null 2>&1 || true

echo "Integration tests passed"
