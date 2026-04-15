COMPOSE := docker compose

.PHONY: help up down up-secure down-secure shell logs test firewall-apply firewall-remove

help:
	@echo "Available targets:"
	@echo "  make up           - Build and start stack (proxy + agent)"
	@echo "  make down         - Stop stack"
	@echo "  make up-secure    - Start stack and apply host firewall policy"
	@echo "  make down-secure  - Remove host firewall policy and stop stack"
	@echo "  make shell        - Open agent shell"
	@echo "  make logs         - Tail proxy/agent logs"
	@echo "  make test         - Run integration tests"
	@echo "  make firewall-apply   - Apply iptables policy (Linux)"
	@echo "  make firewall-remove  - Remove iptables policy (Linux)"

up:
	$(COMPOSE) up -d --build

down:
	$(COMPOSE) down

up-secure: up firewall-apply

down-secure:
	-$(MAKE) firewall-remove
	$(COMPOSE) down

shell:
	$(COMPOSE) run --rm agent

logs:
	$(COMPOSE) logs -f proxy agent

test:
	./scripts/test-integration.sh

firewall-apply:
	./scripts/apply-egress-firewall.sh

firewall-remove:
	./scripts/remove-egress-firewall.sh
