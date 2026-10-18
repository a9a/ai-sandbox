# AI Agent Sandbox (Docker)

Deterministic Docker sandbox for AI coding agents with controlled egress through a proxy container.

## Files

- `docker-compose.yml` - shared proxy, networks, secrets, and Claude/Codex services.
- `docker-compose.docker.yml` - shared rootless Docker daemon and agent tooling overlay.
- `Dockerfile.agent` - shared Dockerfile with targets: `claude`, `claude-docker`, `codex`, `codex-docker`.
- `cli/agent-box` - standalone Claude/Codex workspace launcher.
- `justfile` - development tasks for image builds, tests, and cleanup.
- `Makefile` - fallback development task runner with matching targets.
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
CLAUDE_NODE_IMAGE=node:24-slim@sha256:0e0ff40c39bc087845bfb27465a0df4ea419520094bc35842ff83dd8cbe6f9b6
CODEX_NODE_IMAGE=node:24-slim@sha256:0e0ff40c39bc087845bfb27465a0df4ea419520094bc35842ff83dd8cbe6f9b6
CLAUDE_CODE_VERSION=2.1.280
CODEX_VERSION=0.155.0
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

## Install Launchers

Build and install the standalone host launchers:

```bash
cargo build --manifest-path cli/agent-box/Cargo.toml --release
install -d "$HOME/.local/bin"
install -m 755 cli/agent-box/target/release/claude-box "$HOME/.local/bin/claude-box"
install -m 755 cli/agent-box/target/release/codex-box "$HOME/.local/bin/codex-box"
```

The same binaries work on Linux and macOS when built for the target platform. Ensure `$HOME/.local/bin` is in `PATH`; no repository checkout or `just` installation is required at runtime. Docker with Compose v2 must be available on the host.

## Configure Workspaces

Create the default user configuration:

```bash
mkdir -p "$HOME/.config/ai-sandbox"
cp box.toml.example "$HOME/.config/ai-sandbox/box.toml"
```

Example:

```toml
[defaults]
docker = true

[[workspaces]]
name = "product-x"
docker_ports = "18000-18099"

[[workspaces.mounts]]
name = "app"
host = "/Users/aga/dev/product-x-app"

[[workspaces.mounts]]
name = "api"
host = "/Users/aga/repos/product-x-api"
```

The launcher reads only `~/.config/ai-sandbox/box.toml`. Configured workspaces appear first and the directory from which the launcher was invoked is always appended as the final `Current directory` option. A defaults-only file is therefore valid:

```toml
[defaults]
docker = true
```

Run a launcher from any project directory:

```bash
claude-box
codex-box
```

The selected host directories are mounted under `/home/devops/project/<workspace-name>/<mount-name>`. All configured workspace mounts are also attached at the same paths to one shared DinD daemon so inner bind mounts resolve correctly. Type to fuzzy-search, use arrow keys to refine the selection, and press Enter. Docker support is enabled by default; pass `--no-docker` for the non-Docker stack or `--build` to rebuild images before opening the agent.

Launcher agent homes are isolated by default under `~/.ai-sandbox/claude` and `~/.ai-sandbox/codex`; the launcher creates them with mode `0700`. To explicitly share existing host configuration, set `claude_home` or `codex_home` in `[defaults]`, or export `CLAUDE_HOME_PATH` / `CODEX_HOME_PATH`.

Launcher agent runtimes are scoped by workspace. Reopening the same workspace starts another `exec` session in the existing agent container; opening a different workspace leaves previous containers and sessions running. All Docker-enabled agents use one shared rootless `ai-sandbox-docker-daemon`, Docker cache, Unix socket, `ai-sandbox-proxy`, and `host-dind-forwarder`. Consequently, Docker-enabled agents can inspect and modify each other's inner containers, images, networks, and volumes.

Existing per-workspace DinD containers and volumes from older launcher versions are not migrated into the shared daemon automatically. They can be removed after any required data has been copied or rebuilt.

When `Current directory` is outside configured mounts, the launcher offers two Docker modes:

- use shared Docker immediately without project bind mounts,
- persist the current directory as a DinD mount and recreate the shared daemon.

The first mode supports image pulls, ordinary containers, named volumes, `docker build .`, and Compose projects without host bind mounts. The second mode is required for `docker run -v "$PWD:/app"` and Compose entries such as `.:/app`. Added current-directory mounts are retained in the launcher state, so later sessions reuse them without another restart. If the current directory is already inside a configured mount, full bind-mount support is enabled automatically.

Optional `docker_ports` publishes the same port range through the shared `host-dind-forwarder` to host loopback. The forwarder is the only component connected to its non-internal ingress network; agents and the shared DinD daemon remain on the internal network. Ranges must use unprivileged ports and cannot overlap between workspaces. For example, `docker_ports = "18000-18099"` makes an inner `docker run -p 18080:80 ...` service available at `http://localhost:18080` without exposing it beyond the host.

For workspaces with `docker_ports`, the launcher sets `SANDBOX_DOCKER_PORT_RANGE` in the agent and adds bundled port guidance to the agent's global instruction file. Existing `AGENTS.md` or `CLAUDE.md` content from the selected agent home is preserved in the generated file, and the source file is not modified. The generated instruction applies to every mounted project opened in that workspace runtime. Repository developers can change `instructions/docker-ports.md` and rebuild the launcher to update this embedded guidance.

The generated agent container runs an idle keepalive process. Every interactive Claude/Codex session is started through the matching entrypoint so secrets are loaded from Docker secret files for that session.

`box.toml` is required, but it does not need to contain configured workspaces. The automatic current-directory workspace does not publish DinD ports; add a named workspace with `docker_ports` when host access to inner services is needed.

## Runtime Assets

Compose files, Dockerfiles, proxy configuration, entrypoints, and agent instructions are embedded in each launcher binary. On first use they are materialized into a versioned cache directory:

- Linux: `${XDG_CACHE_HOME:-$HOME/.cache}/ai-sandbox/runtime/`
- macOS: `$HOME/Library/Caches/ai-sandbox/runtime/`

Generated Compose overrides, instruction files, and the retained list of dynamic DinD mounts are stored under the adjacent `ai-sandbox/generated/` directory. Set `AI_SANDBOX_DIR` only during launcher development to use runtime assets from a repository checkout instead of the embedded copy.

Docker-enabled Claude/Codex profiles use one shared rootless Docker daemon and Unix socket (`DOCKER_HOST=unix:///run/user/1000/docker.sock`). Both images install `docker`, `docker compose`, and `helm` from the shared tooling stage in `Dockerfile.agent`. The launcher assigns a workspace-and-mount-specific `COMPOSE_PROJECT_NAME` to interactive sessions to avoid inner Compose collisions. Services published by inner Docker containers are reachable from an agent at `http://docker-daemon:<port>` and, for configured `docker_ports`, from host loopback through `host-dind-forwarder`.

`just` and `make` are development-only helpers:

```bash
just claude-build
just codex-build
just test
just down-all
```

Claude home data (`/home/devops/.claude`) is persisted via host bind mount.
The entrypoint auto-creates `/home/devops/.claude/.claude.json` and links `/home/devops/.claude.json` to it.

- Launcher default: `~/.ai-sandbox/claude`.
- Override with `claude_home` in `box.toml` or `CLAUDE_HOME_PATH`.

Codex home data (`/home/devops/.codex`) is persisted in a host directory bind mount.

- Launcher default: `~/.ai-sandbox/codex`.
- Override with `codex_home` in `box.toml` or `CODEX_HOME_PATH`.

## Build Images

```bash
just claude-build
just codex-build
```

The Rust launchers do not rebuild images by default. Rebuild explicitly with the targets above or pass `--build` to `claude-box` / `codex-box`.

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
production.cloudfront.docker.com
.r2.cloudflarestorage.com
hub.docker.com
github.com
ghcr.io
pkg-containers.githubusercontent.com
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
- Docker communication uses a Unix socket, so it needs no additional network exception.

Remove a previously applied host firewall policy:

```bash
just firewall-remove
```

Scripts require Linux host with `iptables` and running containers.

## Local Tests

Run both integration suites:

```bash
just test
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
  - reports `HIGH` findings without blocking CI,
  - fails on `CRITICAL` findings.

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
