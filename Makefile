COMPOSE_BASE := docker compose -f docker-compose.yml
COMPOSE_CLAUDE := $(COMPOSE_BASE) -f docker-compose.claude.yml
COMPOSE_CLAUDE_DOCKER := $(COMPOSE_CLAUDE) -f docker-compose.agent.docker.yml -f docker-compose.claude.docker.yml
COMPOSE_CODEX := $(COMPOSE_BASE) -f docker-compose.codex.yml
COMPOSE_CODEX_DOCKER := $(COMPOSE_CODEX) -f docker-compose.agent.docker.yml -f docker-compose.codex.docker.yml
COMPOSE_ALL := $(COMPOSE_BASE) -f docker-compose.claude.yml -f docker-compose.codex.yml

.PHONY: help \
	claude-build codex-build claude-up claude-down claude-up-secure claude-down-secure claude-workspace claude-logs \
	claude-docker-up claude-docker-down claude-docker-up-secure claude-docker-down-secure claude-docker-workspace claude-docker-logs \
	codex-up codex-down codex-up-secure codex-down-secure codex-workspace codex-logs \
	codex-docker-up codex-docker-down codex-docker-up-secure codex-docker-down-secure codex-docker-workspace codex-docker-logs \
	test-claude test-codex firewall-apply firewall-remove firewall-apply-claude firewall-apply-claude-docker firewall-apply-codex firewall-apply-codex-docker down-all

help:
	@echo "Available targets:"
	@echo "  make claude-up                 - Start Claude stack"
	@echo "  make claude-workspace          - Enter Claude workspace as devops"
	@echo "  make claude-down               - Stop Claude stack"
	@echo "  make claude-logs               - Follow Claude and proxy logs"
	@echo "  make claude-build              - Build Claude image"
	@echo "  make claude-up-secure          - Start Claude stack and apply host firewall policy"
	@echo "  make claude-down-secure        - Remove firewall policy and stop Claude stack"
	@echo "  make claude-docker-up          - Start Claude stack with sidecar Docker daemon"
	@echo "  make claude-docker-workspace   - Enter Docker-enabled Claude workspace"
	@echo "  make claude-docker-down        - Stop Docker-enabled Claude stack"
	@echo "  make claude-docker-logs        - Follow proxy, Docker daemon, and Claude logs"
	@echo "  make codex-up                  - Start Codex stack"
	@echo "  make codex-workspace           - Enter Codex workspace as devops"
	@echo "  make codex-down                - Stop Codex stack"
	@echo "  make codex-logs                - Follow Codex and proxy logs"
	@echo "  make codex-build               - Build Codex image"
	@echo "  make codex-docker-up           - Start Codex stack with sidecar Docker daemon"
	@echo "  make codex-docker-workspace    - Enter Docker-enabled Codex workspace"
	@echo "  make codex-docker-down         - Stop Docker-enabled Codex stack"
	@echo "  make codex-docker-logs         - Follow proxy, Docker daemon, and Codex logs"
	@echo "  make test                      - Run integration tests for Claude and Codex"
	@echo "  make down-all                  - Stop both Claude and Codex stacks"
	@echo "  Optional: AI_HOME_PATH=/path mounts a custom host workspace at /home/devops/project"

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

claude-workspace:
	$(COMPOSE_CLAUDE) exec --user devops -e HOME=/home/devops -w /home/devops/project claude-agent bash

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

claude-docker-workspace:
	$(COMPOSE_CLAUDE_DOCKER) exec --user devops -e HOME=/home/devops -w /home/devops/project claude-agent bash

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

codex-workspace:
	$(COMPOSE_CODEX) exec --user devops -e HOME=/home/devops -w /home/devops/project codex-agent bash

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

codex-docker-workspace:
	$(COMPOSE_CODEX_DOCKER) exec --user devops -e HOME=/home/devops -w /home/devops/project codex-agent bash

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
