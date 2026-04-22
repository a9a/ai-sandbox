# AI Agent Sandbox (Docker)

Deterministic Docker sandbox for AI coding agents with controlled egress through a proxy container.

## Files

- `docker-compose.yml` - shared base stack (proxy + networks).
- `docker-compose.agent.yml` - shared agent service base used by Claude/Codex via `extends`.
- `docker-compose.agent.docker.yml` - shared Docker sidecar (`dind-rootless`) for Docker-enabled profiles.
- `docker-compose.claude.yml` - Claude agent service.
- `docker-compose.claude.docker.yml` - Claude profile with sidecar Docker daemon.
- `docker-compose.codex.yml` - Codex agent service.
- `docker-compose.codex.docker.yml` - Codex profile with sidecar Docker daemon.
- `Dockerfile.agent` - shared Dockerfile with targets: `claude`, `claude-docker`, `codex`, `codex-docker`.
- `Makefile` - convenience commands for compose, firewall, and tests.
- `.env` - pinned versions and image tags.
- `proxy/squid.conf` - proxy policy (domain allowlist).
- `proxy/allowed-domains.txt` - destination domain allowlist.
- `scripts/claude-entrypoint.sh` - loads Anthropic/GitHub secrets and drops to `devops`.
- `scripts/codex-entrypoint.sh` - loads OpenAI/GitHub secrets and drops to `devops`.
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
CLAUDE_DOCKER_IMAGE_NAME=ai-sandbox-claude-agent-docker:local
CODEX_IMAGE_NAME=ai-sandbox-codex-agent:local
CODEX_DOCKER_IMAGE_NAME=ai-sandbox-codex-agent-docker:local
DOCKER_CLI_IMAGE=docker:27-cli
DOCKER_DIND_IMAGE=docker:27-dind-rootless
HELM_VERSION=v3.18.4
```

## Configure Secrets

All keys are optional.

- If not set, containers still start, but authenticated API calls will fail.
- If set, Compose mounts them as Docker secrets (`/run/secrets/...`) so they are not visible in `docker inspect` env.
- `.env` already defines all of them as empty by default, so `docker compose up` does not fail when they are missing.

Set keys from shell env:

```bash
export ANTHROPIC_API_KEY='...'
export OPENAI_API_KEY='...'
export GITHUB_TOKEN='...'
```

Or load them from local files:

```bash
export ANTHROPIC_API_KEY="$(tr -d '\r\n' < secrets/anthropic_api_key.txt)"
export OPENAI_API_KEY="$(tr -d '\r\n' < secrets/openai_api_key.txt)"
export GITHUB_TOKEN="$(tr -d '\r\n' < secrets/github_token.txt)"
```

## Configure Git Identity

Set optional Git identity in `.env` (or export in shell):

```bash
export GIT_USER_NAME='ai-bot'
export GIT_USER_EMAIL='ai-bot@users.noreply.github.com'
```

When set, both agent entrypoints apply these values via `git config --global` for user `devops` at container start.

If `GITHUB_TOKEN` is present, the entrypoint writes `~/.git-credentials` and configures `credential.helper store` automatically.

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

Claude with Docker-in-Docker sidecar:

```bash
make claude-docker-up
make claude-docker-shell
make claude-docker-new
make claude-docker-up-secure
make claude-docker-down-secure
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

Docker-enabled Claude/Codex profiles use a rootless Docker daemon sidecar (`DOCKER_DIND_IMAGE`, default `docker:27-dind-rootless`) and Unix socket communication (`DOCKER_HOST=unix:///run/user/1000/docker.sock`).
Both Docker-enabled agent images install `docker`, `docker compose`, and `helm` from one shared tooling stage in `Dockerfile.agent`.
On Docker Desktop and other nested-container environments, `docker-daemon` runs with `privileged: true` so inner containers can mount `/proc` and start correctly.

Backward-compatible aliases (`up`, `shell`, `down-secure`) default to Claude.

Agent project directory (`/home/devops/project`) is mounted from the host.

- Default path is `.` (where `docker compose` is started).
- Optional: set `AI_HOME_PATH` in `.env` (example: `/path/to/project`) to override.
- In `claude-docker` and `codex-docker` profiles, the same path is mounted into `docker-daemon` so bind mounts work via remote `DOCKER_HOST`.

Claude home data (`/home/devops/.claude`) and session state file (`/home/devops/.claude.json`) are persisted via host bind mounts.

- Default path is `$PWD/.claude` (where `docker compose` is started).
- Optional: set `CLAUDE_HOME_PATH` in `.env` (example: `/path/to/claude-home`) to override.
- For file mount compatibility, ensure `${CLAUDE_HOME_PATH}/.claude.json` exists on host (an empty file is enough).

Codex home data (`/home/devops/.codex`) is persisted in a host directory bind mount.

- Default path is `$PWD/.codex` (where `docker compose` is started).
- Optional: set `CODEX_HOME_PATH` in `.env` (example: `/path/to/codex-home`) to override.

## Build Images

```bash
make claude-build
make codex-build
```

## Proxy Allowlist (Domain ACL)

Edit `proxy/allowed-domains.txt` (one domain per line; use `.domain.tld` for subdomain wildcard suffixes).

Current baseline:

```txt
api.anthropic.com
console.anthropic.com
platform.claude.com
api.openai.com
auth.openai.com
chatgpt.com
auth.docker.io
index.docker.io
registry.docker.io
registry-1.docker.io
production.cloudflare.docker.com
.r2.cloudflarestorage.com
hub.docker.com
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
- In `claude-docker-*-secure` and `codex-docker-*-secure` modes no extra network exception is needed for Docker (Unix socket is used).

Apply/remove host firewall:

```bash
make firewall-apply-claude
make firewall-apply-claude-docker
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
