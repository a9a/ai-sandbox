#!/usr/bin/env bash
set -euo pipefail

CHAIN="AI_SANDBOX_EGRESS"

if ! command -v iptables >/dev/null 2>&1; then
  echo "iptables not found on host. This script currently supports Linux hosts."
  exit 1
fi

while sudo iptables -C DOCKER-USER -j "$CHAIN" 2>/dev/null; do
  sudo iptables -D DOCKER-USER -j "$CHAIN"
done

sudo iptables -F "$CHAIN" 2>/dev/null || true
sudo iptables -X "$CHAIN" 2>/dev/null || true

echo "Removed firewall policy chain $CHAIN"
