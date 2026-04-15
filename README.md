# AI Agent Sandbox (Docker)

Minimal, deterministic Docker sandbox for an AI agent CLI with controlled egress through a proxy container.

## Files

- `Dockerfile` - base image, system packages, pinned agent install, non-root user.
- `.env` - single place for version/tag changes.
- `build.sh` - builds image using values from `.env`.
- `docker-compose.yml` - runs `agent` + `proxy` with network isolation.
- `proxy/squid.conf` - proxy policy (allow only listed destination domains).
- `proxy/allowed-domains.txt` - destination domain allowlist.
- `scripts/apply-egress-firewall.sh` - required host firewall (`DOCKER-USER`) enforcement.
- `scripts/remove-egress-firewall.sh` - removes the host firewall chain.

## Configure Versions

Edit `.env`:

```env
NODE_IMAGE=node:20-slim@sha256:87ef9545464152504958f33887010424a106f0f29c4202353e6b206981f3d81b
CLAUDE_CODE_VERSION=0.2.9
IMAGE_NAME=ai-agent-sandbox:local
```

## Build

```bash
./build.sh
```

## Run (Direct Docker)

Mount current project into container workspace:

```bash
docker run --rm -it \
  -v "$PWD:/home/devops/project" \
  -e ANTHROPIC_API_KEY="$ANTHROPIC_API_KEY" \
  ai-agent-sandbox:local --version
```

If you changed `IMAGE_NAME`, use that value instead of `ai-agent-sandbox:local`.

## Run With Proxy Egress Control

Build and start services:

```bash
docker compose up -d --build
```

Apply hard host firewall policy (`agent` can only connect to `proxy:3128`):

```bash
./scripts/apply-egress-firewall.sh
```

Run an interactive agent shell:

```bash
docker compose run --rm agent
```

Stop services:

```bash
docker compose down
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
- Domain ACL avoids brittle provider-wide IP ranges.
- `docker compose` automatically reads variables from `.env` in the project root.
- Squid denies `CONNECT` to literal IP targets, reducing proxy-bypass attempts.

## Host Firewall

Apply `iptables` policy so agent traffic is allowed only to the proxy on port `3128`:

```bash
./scripts/apply-egress-firewall.sh
```

Remove policy:

```bash
./scripts/remove-egress-firewall.sh
```

The scripts use `sudo`, require running containers (`ai-sandbox-agent`, `ai-sandbox-proxy`) to resolve current container IPs, and currently target Linux hosts with `iptables`.

## Update Policy

- Keep `NODE_IMAGE` pinned to a digest for deterministic builds.
- Bump digest periodically for security patches.
- Keep `CLAUDE_CODE_VERSION` pinned to a specific version for reproducibility.
