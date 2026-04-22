# Repository Guidelines

## Project Structure & Module Organization

This repository defines a Docker sandbox for AI coding agents. Top-level
Compose files define the stacks: `docker-compose.yml` is the shared base,
`docker-compose.claude.yml` and `docker-compose.codex.yml` add agent services,
and `docker-compose.codex.docker.yml` enables the Docker sidecar profile.
Agent images are built from `Dockerfile.claude`,
`Dockerfile.codex`, and `Dockerfile.codex.docker`. Proxy policy lives in
`proxy/`, operational scripts live in `scripts/`, and CI definitions live in
`.github/workflows/`. Integration test fixtures are under `.ci/`.

## Build, Test, and Development Commands

Run `make help` to list supported workflows. Common commands:

- `make claude-build` / `make codex-build`: build the agent images.
- `make claude-up-secure` / `make codex-up-secure`: start a stack and apply the
  host egress firewall.
- `make codex-docker-up`: start Codex with the rootless Docker sidecar.
- `make claude-shell` / `make codex-shell`: attach to a running agent container.
- `make test`: run both Claude and Codex integration suites.
- `./scripts/test-integration.sh codex`: run one integration suite directly.

## Coding Style & Naming Conventions

Use Bash for repository scripts and match existing conventions in
`scripts/*.sh`. Prefer explicit variables, quoted expansions, and clear error
handling. Compose services, container names, and Make targets use lowercase,
hyphenated names such as
`codex-docker-up-secure`. Keep Docker image versions and digests pinned in
`.env`; avoid unpinned `latest` tags.

## Testing Guidelines

Integration tests are shell-based and run through
`scripts/test-integration.sh`. They validate proxy allowlist behavior, literal
IP blocking, secret mounting, and secret absence from `docker inspect`
environment output. Add tests when changing Compose networking, proxy rules,
entrypoints, secret handling, or firewall scripts. CI also runs shell syntax
checks, `shellcheck`, Compose validation, image builds, and security scans.

## Commit & Pull Request Guidelines

Recent history uses short imperative commits, often Conventional Commit
prefixes such as `feat:` and `fix:`. Keep messages focused, for example
`feat: add git auth support` or `fix: tighten proxy test`. Pull requests should
include a concise description, affected stack or script, linked issues when
available, and verification commands. Include screenshots only for terminal or
CI output that clarifies a failure or behavior change.

## Security & Configuration Tips

Do not commit real secrets. Keep local credentials in shell environment
variables or files under `secrets/`, which is intended for local secret inputs.
Review `proxy/allowed-domains.txt` carefully because it defines allowed egress.
After changing proxy or firewall behavior, run `make test` and verify the
secure targets still restrict agent traffic through the proxy.
