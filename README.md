# AI Agent Sandbox (Docker)

Minimal, deterministic Docker sandbox for an AI agent CLI with controlled egress through a proxy container.

## Files

- `Dockerfile` - base image, system packages, pinned agent install, non-root user.
- `FUTURE_IMPROVEMENTS.md` - backlog of planned hardening/maintenance work.
- `Makefile` - convenience commands for compose, firewall, and tests.
- `.env` - single place for version/tag changes.
- `build.sh` - builds image using values from `.env`.
- `docker-compose.yml` - runs `agent` + `proxy` with network isolation.
- `scripts/agent-entrypoint.sh` - loads `ANTHROPIC_API_KEY` from `/run/secrets/anthropic_api_key` and drops privileges to `devops`.
- `proxy/squid.conf` - proxy policy (allow only listed destination domains).
- `proxy/allowed-domains.txt` - destination domain allowlist.
- `secrets/anthropic_api_key.txt.example` - template for local secret file.
- `scripts/apply-egress-firewall.sh` - required host firewall (`DOCKER-USER`) enforcement.
- `scripts/remove-egress-firewall.sh` - removes the host firewall chain.

## Configure Versions

Edit `.env`:

```env
NODE_IMAGE=node:20-slim@sha256:f93745c153377ee2fbbdd6e24efcd03cd2e86d6ab1d8aa9916a3790c40313a55
CLAUDE_CODE_VERSION=0.2.9
IMAGE_NAME=ai-agent-sandbox:local
```

## Build

```bash
./build.sh
```

## Make Targets

```bash
make help
```

Most common:

```bash
make up-secure
make shell
make down-secure
```

## Local Tests

Run integration tests for:
- proxy allowlist (`allowed` and `blocked` domain behavior),
- literal IP blocking through proxy,
- Docker Secrets mounting and runtime key loading.

```bash
make test
```

## Configure API Secret

Create local secret file (not tracked by git):

```bash
cp secrets/anthropic_api_key.txt.example secrets/anthropic_api_key.txt
chmod 600 secrets/anthropic_api_key.txt
```

Then replace file content with your real key.
`chmod 600` is supported with this setup.

## Run (Direct Docker)

Mount current project into container workspace:

```bash
docker run --rm -it \
  -v "$PWD:/home/devops/project" \
  -v "$PWD/secrets/anthropic_api_key.txt:/run/secrets/anthropic_api_key:ro" \
  ai-agent-sandbox:local --version
```

If you changed `IMAGE_NAME`, use that value instead of `ai-agent-sandbox:local`.

## Run With Proxy Egress Control

Build and start services:

```bash
make up
```

Apply hard host firewall policy (`agent` can only connect to `proxy:3128`):

```bash
make firewall-apply
```

Run an interactive agent shell:

```bash
make shell
```

Stop services:

```bash
make down
```

## Configure Proxy Allowlist (Domain ACL)

Edit `proxy/allowed-domains.txt` and keep one destination domain per line:

Example:

```txt
api.anthropic.com
console.anthropic.com
```

After changes, restart proxy:

```bash
docker compose up -d --build proxy
```

Notes:
- `agent` is connected only to `agent_net` (`internal: true`) and cannot reach internet directly.
- `proxy` is connected to both `agent_net` and `egress_net`, so internet egress happens only via proxy.
- `proxy` has a healthcheck and `agent` waits for `service_healthy` before startup.
- Domain ACL avoids brittle provider-wide IP ranges.
- `docker compose` automatically reads variables from `.env` in the project root.
- `docker compose` reads secret `./secrets/anthropic_api_key.txt` and mounts it to `/run/secrets/anthropic_api_key`.
- Squid denies `CONNECT` to literal IP targets, reducing proxy-bypass attempts.

## Host Firewall

Apply `iptables` policy so agent traffic is allowed only to the proxy on port `3128`:

```bash
make firewall-apply
```

Remove policy:

```bash
make firewall-remove
```

The scripts use `sudo`, require running containers (`ai-sandbox-agent`, `ai-sandbox-proxy`) to resolve current container IPs, and currently target Linux hosts with `iptables`.

## GitHub Actions

- `CI` workflow (`.github/workflows/ci.yml`)
  - shell checks (`bash -n`, `shellcheck`),
  - compose validation,
  - image builds,
  - integration tests (`./scripts/test-integration.sh`).
- `Security Scan` workflow (`.github/workflows/security.yml`)
  - daily Trivy scan of repo and both images,
  - fails on `HIGH`/`CRITICAL`.

## Renovate

Configuration is in `.github/renovate.json`.

Schedule:
- daily on weekdays for Docker digest updates,
- weekly on Monday for patch/minor updates,
- weekly on Monday for major updates (separate PRs).

To enable it, install the official Renovate GitHub App on this repository.

## Update Policy

- Keep `NODE_IMAGE` pinned to a digest for deterministic builds.
- Bump digest periodically for security patches.
- Keep `CLAUDE_CODE_VERSION` pinned to a specific version for reproducibility.
