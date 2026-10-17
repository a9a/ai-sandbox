set dotenv-load := true

compose_all := "docker compose -f docker-compose.yml -f docker-compose.docker.yml --profile claude --profile codex --profile ports"

default:
    @just --list

claude-build:
    ./build.sh

codex-build:
    ./build-codex.sh

firewall-remove:
    ./scripts/remove-egress-firewall.sh

test: test-claude test-codex

test-claude:
    ./scripts/test-integration.sh claude

test-codex:
    ./scripts/test-integration.sh codex

down-all:
    -./scripts/remove-egress-firewall.sh
    {{compose_all}} down
