COMPOSE_BASE := docker compose -f docker-compose.yml
COMPOSE_CLAUDE := $(COMPOSE_BASE) -f docker-compose.claude.yml
COMPOSE_CODEX := $(COMPOSE_BASE) -f docker-compose.codex.yml
COMPOSE_ALL := $(COMPOSE_BASE) -f docker-compose.claude.yml -f docker-compose.codex.yml

.PHONY: help \
	up down up-secure down-secure shell logs build test \
	claude-build codex-build claude-up claude-down claude-up-secure claude-down-secure claude-shell claude-logs \
	codex-up codex-down codex-up-secure codex-down-secure codex-shell codex-logs \
	test-claude test-codex firewall-apply firewall-remove firewall-apply-claude firewall-apply-codex down-all

help:
	@echo "Available targets:"
	@echo "  make claude-up-secure   - Start Claude stack and apply host firewall policy"
	@echo "  make claude-shell       - Open Claude agent shell"
	@echo "  make claude-down-secure - Remove firewall policy and stop Claude stack"
	@echo "  make codex-up-secure    - Start Codex stack and apply host firewall policy"
	@echo "  make codex-shell        - Open Codex agent shell"
	@echo "  make codex-down-secure  - Remove firewall policy and stop Codex stack"
	@echo "  make test               - Run integration tests for Claude and Codex"
	@echo "  make down-all           - Stop both Claude and Codex stacks"

# Backward-compatible aliases (Claude as default)
up: claude-up

down: claude-down

up-secure: claude-up-secure

down-secure: claude-down-secure

shell: claude-shell

logs: claude-logs

build: claude-build

test: test-claude test-codex

claude-build:
	./build.sh

codex-build:
	./build-codex.sh

claude-up:
	$(COMPOSE_CLAUDE) up -d --build

claude-down:
	$(COMPOSE_CLAUDE) down

claude-up-secure: claude-up firewall-apply-claude

claude-down-secure:
	-$(MAKE) firewall-remove
	$(COMPOSE_CLAUDE) down

claude-shell:
	$(COMPOSE_CLAUDE) run --rm claude-agent

claude-logs:
	$(COMPOSE_CLAUDE) logs -f proxy claude-agent

codex-up:
	$(COMPOSE_CODEX) up -d --build

codex-down:
	$(COMPOSE_CODEX) down

codex-up-secure: codex-up firewall-apply-codex

codex-down-secure:
	-$(MAKE) firewall-remove
	$(COMPOSE_CODEX) down

codex-shell:
	$(COMPOSE_CODEX) run --rm codex-agent

codex-logs:
	$(COMPOSE_CODEX) logs -f proxy codex-agent

firewall-apply: firewall-apply-claude

firewall-apply-claude:
	./scripts/apply-egress-firewall.sh ai-sandbox-claude-agent ai-sandbox-proxy

firewall-apply-codex:
	./scripts/apply-egress-firewall.sh ai-sandbox-codex-agent ai-sandbox-proxy

firewall-remove:
	./scripts/remove-egress-firewall.sh

test-claude:
	./scripts/test-integration.sh claude

test-codex:
	./scripts/test-integration.sh codex

down-all:
	-$(MAKE) firewall-remove
	$(COMPOSE_ALL) down
