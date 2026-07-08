set dotenv-load := true

compose_base := "docker compose -f docker-compose.yml"
compose_claude := compose_base + " -f docker-compose.claude.yml"
compose_claude_docker := compose_claude + " -f docker-compose.agent.docker.yml -f docker-compose.claude.docker.yml"
compose_codex := compose_base + " -f docker-compose.codex.yml"
compose_codex_docker := compose_codex + " -f docker-compose.agent.docker.yml -f docker-compose.codex.docker.yml"
compose_all := compose_base + " -f docker-compose.claude.yml -f docker-compose.codex.yml"

default:
    @just --list

claude-build:
    ./build.sh

codex-build:
    ./build-codex.sh

claude-up:
    {{compose_claude}} up -d --build

claude-down:
    {{compose_claude}} down

claude-up-secure: claude-up firewall-apply-claude

claude-down-secure:
    -./scripts/remove-egress-firewall.sh
    {{compose_claude}} down

claude-workspace:
    {{compose_claude}} exec --user devops -e HOME=/home/devops -w /home/devops/project claude-agent bash

claude-logs:
    {{compose_claude}} logs -f proxy claude-agent

claude-docker-up:
    {{compose_claude_docker}} up -d --build

claude-docker-down:
    {{compose_claude_docker}} down

claude-docker-up-secure: claude-docker-up firewall-apply-claude-docker

claude-docker-down-secure:
    -./scripts/remove-egress-firewall.sh
    {{compose_claude_docker}} down

claude-docker-workspace:
    {{compose_claude_docker}} exec --user devops -e HOME=/home/devops -w /home/devops/project claude-agent bash

claude-docker-logs:
    {{compose_claude_docker}} logs -f proxy docker-daemon claude-agent

codex-up:
    {{compose_codex}} up -d --build

codex-down:
    {{compose_codex}} down

codex-up-secure: codex-up firewall-apply-codex

codex-down-secure:
    -./scripts/remove-egress-firewall.sh
    {{compose_codex}} down

codex-workspace:
    {{compose_codex}} exec --user devops -e HOME=/home/devops -w /home/devops/project codex-agent bash

codex-logs:
    {{compose_codex}} logs -f proxy codex-agent

codex-docker-up:
    {{compose_codex_docker}} up -d --build

codex-docker-down:
    {{compose_codex_docker}} down

codex-docker-up-secure: codex-docker-up firewall-apply-codex-docker

codex-docker-down-secure:
    -./scripts/remove-egress-firewall.sh
    {{compose_codex_docker}} down

codex-docker-workspace:
    {{compose_codex_docker}} exec --user devops -e HOME=/home/devops -w /home/devops/project codex-agent bash

codex-docker-logs:
    {{compose_codex_docker}} logs -f proxy docker-daemon codex-agent

firewall-apply: firewall-apply-claude

firewall-apply-claude:
    ./scripts/apply-egress-firewall.sh ai-sandbox-claude-agent ai-sandbox-proxy

firewall-apply-claude-docker:
    ./scripts/apply-egress-firewall.sh ai-sandbox-claude-agent ai-sandbox-proxy

firewall-apply-codex:
    ./scripts/apply-egress-firewall.sh ai-sandbox-codex-agent ai-sandbox-proxy

firewall-apply-codex-docker:
    ./scripts/apply-egress-firewall.sh ai-sandbox-codex-agent ai-sandbox-proxy

firewall-remove:
    ./scripts/remove-egress-firewall.sh

test: test-claude test-codex

test-claude:
    ./scripts/test-integration.sh claude

test-codex:
    ./scripts/test-integration.sh codex

down-all:
    -./scripts/remove-egress-firewall.sh
    -{{compose_claude_docker}} down
    -{{compose_codex_docker}} down
    {{compose_all}} down
