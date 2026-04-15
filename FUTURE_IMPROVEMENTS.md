# Future Improvements

Last updated: 2026-04-15

This file tracks non-blocking improvements we intentionally postponed.

## 1) Proxy Base Image Hardening (POC)

Status: Planned
Priority: Medium

Goal:
- Evaluate `alpine` and/or `wolfi` base images for `proxy` to reduce image size and CVE surface.

Acceptance criteria:
- Squid works with current `Domain ACL` policy.
- Healthcheck still works.
- No regression in integration tests (`scripts/test-integration.sh`).
- Trivy results are equal or better than current Debian-based image.

## 2) Firewall Validation in CI

Status: Planned
Priority: Medium

Goal:
- Add full `iptables` enforcement tests on a Linux self-hosted runner.

Why:
- GitHub-hosted runners are good for container integration tests, but host firewall behavior should be validated separately.

## 3) Secrets Runtime Simplification

Status: Planned
Priority: Low

Goal:
- Revisit running `agent` fully non-root if Docker runtime reliably supports secret `uid/gid/mode` mapping for this environment.

Current constraint:
- In current Docker Compose runtime, secret ownership/mode mapping fields are ignored.

## 4) Supply-Chain Enhancements

Status: Planned
Priority: Low

Goal:
- Add SBOM export and image signing/attestation (e.g. `cosign`) for release builds.

## 5) Renovate Policy Tuning

Status: Planned
Priority: Low

Goal:
- Consider auto-merge only for low-risk digest updates after CI + security checks pass.
