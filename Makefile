COMPOSE_BASE := docker compose -f docker-compose.yml
COMPOSE_CLAUDE := $(COMPOSE_BASE) -f docker-compose.claude.yml
COMPOSE_CLAUDE_DOCKER := $(COMPOSE_CLAUDE) -f docker-compose.claude.docker.yml
COMPOSE_CODEX := $(COMPOSE_BASE) -f docker-compose.codex.yml
COMPOSE_CODEX_DOCKER := $(COMPOSE_CODEX) -f docker-compose.codex.docker.yml
COMPOSE_ALL := $(COMPOSE_BASE) -f docker-compose.claude.yml -f docker-compose.codex.yml

.PHONY: help \
	up down up-secure down-secure shell logs build test \
	claude-build codex-build claude-up claude-down claude-up-secure claude-down-secure claude-shell claude-new claude-logs \
	claude-docker-up claude-docker-down claude-docker-up-secure claude-docker-down-secure claude-docker-shell claude-docker-new claude-docker-logs \
	codex-up codex-down codex-up-secure codex-down-secure codex-shell codex-new codex-logs \
	codex-docker-up codex-docker-down codex-docker-up-secure codex-docker-down-secure codex-docker-shell codex-docker-new codex-docker-logs \
	test-claude test-codex firewall-apply firewall-remove firewall-apply-claude firewall-apply-claude-docker firewall-apply-codex firewall-apply-codex-docker down-all

help:
	@echo "Available targets:"
	@echo "  make claude-up-secure   - Start Claude stack and apply host firewall policy"
	@echo "  make claude-shell       - Attach to running Claude agent (exec claude)"
	@echo "  make claude-new         - Run a new one-off Claude instance (--rm)"
	@echo "  make claude-down-secure - Remove firewall policy and stop Claude stack"
	@echo "  make claude-docker-up   - Start Claude stack with sidecar Docker daemon"
	@echo "  make claude-docker-shell - Attach to running Claude (Docker-enabled profile)"
	@echo "  make claude-docker-new  - Run one-off Claude (Docker-enabled profile)"
	@echo "                           Optional: CLAUDE_HOME_PATH=/path make claude-docker-new"
	@echo "  make claude-docker-up-secure   - Start Docker-enabled Claude stack with firewall"
	@echo "  make claude-docker-down-secure - Stop Docker-enabled Claude stack and remove firewall"
	@echo "  make codex-up           - Build and start Codex stack (proxy + agent)"
	@echo "  make codex-shell        - Attach to running Codex agent (exec codex)"
	@echo "  make codex-new          - Run a new one-off Codex instance (--rm)"
	@echo "                           Optional: CODEX_HOME_PATH=/path make codex-new"
	@echo "  make codex-up-secure    - Start Codex stack and apply host firewall policy"
	@echo "  make codex-down-secure  - Remove firewall policy and stop Codex stack"
	@echo "  make codex-docker-up    - Start Codex stack with sidecar Docker daemon"
	@echo "  make codex-docker-shell - Attach to running Codex (Docker-enabled profile)"
	@echo "  make codex-docker-new   - Run one-off Codex (Docker-enabled profile)"
	@echo "                           Optional: CODEX_HOME_PATH=/path make codex-docker-new"
	@echo "  make codex-docker-up-secure   - Start Docker-enabled Codex stack with firewall"
	@echo "  make codex-docker-down-secure - Stop Docker-enabled stack and remove firewall"
	@echo "  make test               - Run integration tests for Claude and Codex"
	@echo "  make down-all           - Stop both Claude and Codex stacks"
	@echo "  Optional: AI_HOME_PATH=/path mounts a custom host project directory"

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
	$(COMPOSE_CLAUDE) exec --user devops -e HOME=/home/devops claude-agent claude

claude-new:
	$(COMPOSE_CLAUDE) run --rm claude-agent claude

claude-logs:
	$(COMPOSE_CLAUDE) logs -f proxy claude-agent

claude-docker-up:
	$(COMPOSE_CLAUDE_DOCKER) up -d --build

claude-docker-down:
	$(COMPOSE_CLAUDE_DOCKER) down

claude-docker-up-secure: claude-docker-up firewall-apply-claude-docker

claude-docker-down-secure:
	-$(MAKE) firewall-remove
	$(COMPOSE_CLAUDE_DOCKER) down

claude-docker-shell:
	$(COMPOSE_CLAUDE_DOCKER) exec --user devops -e HOME=/home/devops claude-agent claude

claude-docker-new:
	CLAUDE_HOME_PATH="$(CLAUDE_HOME_PATH)" $(COMPOSE_CLAUDE_DOCKER) run --rm claude-agent claude

claude-docker-logs:
	$(COMPOSE_CLAUDE_DOCKER) logs -f proxy docker-daemon claude-agent

codex-up:
	$(COMPOSE_CODEX) up -d --build

codex-down:
	$(COMPOSE_CODEX) down

codex-up-secure: codex-up firewall-apply-codex

codex-down-secure:
	-$(MAKE) firewall-remove
	$(COMPOSE_CODEX) down

codex-shell:
	$(COMPOSE_CODEX) exec --user devops -e HOME=/home/devops codex-agent codex

codex-new:
	CODEX_HOME_PATH="$(CODEX_HOME_PATH)" $(COMPOSE_CODEX) run --rm codex-agent codex

codex-logs:
	$(COMPOSE_CODEX) logs -f proxy codex-agent

codex-docker-up:
	$(COMPOSE_CODEX_DOCKER) up -d --build

codex-docker-down:
	$(COMPOSE_CODEX_DOCKER) down

codex-docker-up-secure: codex-docker-up firewall-apply-codex-docker

codex-docker-down-secure:
	-$(MAKE) firewall-remove
	$(COMPOSE_CODEX_DOCKER) down

codex-docker-shell:
	$(COMPOSE_CODEX_DOCKER) exec --user devops -e HOME=/home/devops codex-agent codex

codex-docker-new:
	CODEX_HOME_PATH="$(CODEX_HOME_PATH)" $(COMPOSE_CODEX_DOCKER) run --rm codex-agent codex

codex-docker-logs:
	$(COMPOSE_CODEX_DOCKER) logs -f proxy docker-daemon codex-agent

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

test-claude:
	./scripts/test-integration.sh claude

test-codex:
	./scripts/test-integration.sh codex

down-all:
	-$(MAKE) firewall-remove
	-$(COMPOSE_CLAUDE_DOCKER) down
	-$(COMPOSE_CODEX_DOCKER) down
	$(COMPOSE_ALL) down
