# AI Agent Sandbox (Docker)

Deterministic Docker sandbox for AI coding agents with controlled egress through a proxy container.

## Files

- `docker-compose.yml` - shared base stack (proxy + networks).
- `docker-compose.claude.yml` - Claude agent service.
- `docker-compose.codex.yml` - Codex agent service.
- `docker-compose.codex.docker.yml` - Codex profile with sidecar Docker daemon.
- `Dockerfile.claude` - Claude agent image.
- `Dockerfile.codex` - Codex agent image.
- `Dockerfile.codex.docker` - Codex agent image with Docker CLI only.
- `Makefile` - convenience commands for compose, firewall, and tests.
- `.env` - pinned versions and image tags.
- `proxy/squid.conf` - proxy policy (domain allowlist).
- `proxy/allowed-domains.txt` - destination domain allowlist.
- `scripts/claude-entrypoint.sh` - loads Anthropic secret and drops to `devops`.
- `scripts/codex-entrypoint.sh` - loads OpenAI secret and drops to `devops`.
- `scripts/apply-egress-firewall.sh` - host firewall policy (`DOCKER-USER`).
- `scripts/remove-egress-firewall.sh` - removes host firewall policy.
- `scripts/test-integration.sh` - integration tests (`claude` or `codex`).
- `FUTURE_IMPROVEMENTS.md` - backlog of postponed improvements.

## Configure Versions

Edit `.env`:

```env
CLAUDE_NODE_IMAGE=node:24-slim@sha256:b506e7321f176aae77317f99d67a24b272c1f09f1d10f1761f2773447d8da26c
CODEX_NODE_IMAGE=node:24-slim@sha256:b506e7321f176aae77317f99d67a24b272c1f09f1d10f1761f2773447d8da26c
CLAUDE_CODE_VERSION=2.1.109
CODEX_VERSION=0.121.0
CLAUDE_IMAGE_NAME=ai-sandbox-claude-agent:local
CODEX_IMAGE_NAME=ai-sandbox-codex-agent:local
CODEX_DOCKER_IMAGE_NAME=ai-sandbox-codex-agent-docker:local
DOCKER_CLI_IMAGE=docker:27-cli
```

## Configure Secrets

Both keys are optional.

- If not set, containers still start, but authenticated API calls will fail.
- If set, Compose mounts them as Docker secrets (`/run/secrets/...`) so they are not visible in `docker inspect` env.
- `.env` already defines both as empty by default, so `docker compose up` does not fail when they are missing.

Set keys from shell env:

```bash
export ANTHROPIC_API_KEY='...'
export OPENAI_API_KEY='...'
```

Or load them from local files:

```bash
export ANTHROPIC_API_KEY="$(tr -d '\r\n' < secrets/anthropic_api_key.txt)"
export OPENAI_API_KEY="$(tr -d '\r\n' < secrets/openai_api_key.txt)"
```

## Make Targets

Show all targets:

```bash
make help
```

Most common (Claude):

```bash
make claude-up-secure
make claude-shell
make claude-new
make claude-down-secure
```

Most common (Codex):

```bash
make codex-up
make codex-shell
make codex-new
make codex-up-secure
make codex-down-secure
```

Codex with Docker-in-Docker sidecar:

```bash
make codex-docker-up
make codex-docker-shell
make codex-docker-new
make codex-docker-up-secure
make codex-docker-down-secure
```

This profile uses a rootless Docker daemon sidecar (`docker:dind-rootless`) and Unix socket communication (`DOCKER_HOST=unix:///run/user/1000/docker.sock`).

Backward-compatible aliases (`up`, `shell`, `down-secure`) default to Claude.

Codex home data (`/home/devops/.codex`) is persisted in a host directory bind mount.

- Default path is `$PWD/.codex` (where `docker compose` is started).
- Optional: set `CODEX_HOME_PATH` in `.env` (example: `/path/to/codex-home`) to override.

## Build Images

```bash
make claude-build
make codex-build
```

## Proxy Allowlist (Domain ACL)

Edit `proxy/allowed-domains.txt` (one domain per line).

Current baseline:

```txt
api.anthropic.com
console.anthropic.com
platform.claude.com
api.openai.com
auth.openai.com
chatgpt.com
auth.docker.io
registry-1.docker.io
production.cloudflare.docker.com
```

After changes:

```bash
docker compose -f docker-compose.yml up -d --build proxy
```

## Egress Model

- Agents run on `agent_net` (`internal: true`) and cannot egress directly.
- Proxy is attached to both `agent_net` and `egress_net`.
- Squid denies `CONNECT` to literal IP targets.
- Use host firewall for hard enforcement (`agent -> proxy:3128` only).
- In `codex-docker-*-secure` mode no extra network exception is needed for Docker (Unix socket is used).

Apply/remove host firewall:

```bash
make firewall-apply-claude
make firewall-apply-codex
make firewall-apply-codex-docker
make firewall-remove
```

Scripts require Linux host with `iptables` and running containers.

## Local Tests

Run both integration suites:

```bash
make test
```

Or run one:

```bash
./scripts/test-integration.sh claude
./scripts/test-integration.sh codex
```

Covered checks:

- proxy allowlist behavior (`allowed`/`blocked` domains),
- literal IP blocking via proxy,
- secret file mounted and loaded into runtime env,
- secret env var not present in `docker inspect` env list.

## GitHub Actions

- `.github/workflows/ci.yml`
  - shell checks (`bash -n`, `shellcheck`),
  - compose validation,
  - build of proxy + Claude + Codex images,
  - integration tests for both agents.
- `.github/workflows/security.yml`
  - daily Trivy scan of repo + proxy image + Claude image + Codex image,
  - fails on `HIGH`/`CRITICAL`.

## Renovate

Configuration: `.github/renovate.json`.

Policy:

- daily weekdays for Docker digest updates,
- weekly Monday for patch/minor updates,
- weekly Monday for major updates.

## Update Policy

- Keep base images pinned by digest.
- Keep CLI versions pinned.
- Periodically refresh digests and rerun CI/security scans.
