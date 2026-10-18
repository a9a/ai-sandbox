COMPOSE_ALL := docker compose -f docker-compose.yml -f docker-compose.docker.yml --profile claude --profile codex --profile docker --profile ports

.PHONY: help claude-build codex-build test test-claude test-codex firewall-remove down-all

help:
	@echo "Available targets:"
	@echo "  make claude-build              - Build Claude image"
	@echo "  make codex-build               - Build Codex image"
	@echo "  make test                      - Run integration tests for Claude and Codex"
	@echo "  make firewall-remove           - Remove the host firewall policy"
	@echo "  make down-all                  - Remove the base development stack"

test: test-claude test-codex

claude-build:
	./build.sh

codex-build:
	./build-codex.sh

firewall-remove:
	./scripts/remove-egress-firewall.sh

test-claude:
	./scripts/test-integration.sh claude

test-codex:
	./scripts/test-integration.sh codex

down-all:
	-$(MAKE) firewall-remove
	$(COMPOSE_ALL) down
