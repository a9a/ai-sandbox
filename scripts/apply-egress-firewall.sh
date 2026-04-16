#!/usr/bin/env bash
set -euo pipefail

AGENT_CONTAINER="${1:-ai-sandbox-claude-agent}"
PROXY_CONTAINER="${2:-ai-sandbox-proxy}"
CHAIN="AI_SANDBOX_EGRESS"

if ! command -v iptables >/dev/null 2>&1; then
  echo "iptables not found on host. This script currently supports Linux hosts."
  exit 1
fi

agent_networks="$(docker inspect -f '{{range $k, $v := .NetworkSettings.Networks}}{{printf "%s %s\n" $k $v.IPAddress}}{{end}}' "$AGENT_CONTAINER")"
proxy_networks="$(docker inspect -f '{{range $k, $v := .NetworkSettings.Networks}}{{printf "%s %s\n" $k $v.IPAddress}}{{end}}' "$PROXY_CONTAINER")"

agent_ip=""
proxy_ip=""
shared_network=""

while read -r network_name candidate_agent_ip; do
  [[ -z "${network_name}" ]] && continue
  candidate_proxy_ip="$(awk -v n="$network_name" '$1 == n {print $2}' <<<"$proxy_networks")"
  if [[ -n "${candidate_proxy_ip}" ]]; then
    shared_network="$network_name"
    agent_ip="$candidate_agent_ip"
    proxy_ip="$candidate_proxy_ip"
    break
  fi
done <<<"$agent_networks"

if [[ -z "${agent_ip}" || -z "${proxy_ip}" || -z "${shared_network}" ]]; then
  echo "Could not find a shared Docker network between $AGENT_CONTAINER and $PROXY_CONTAINER."
  echo "Are both containers running?"
  exit 1
fi

sudo iptables -N "$CHAIN" 2>/dev/null || true
sudo iptables -F "$CHAIN"

if ! sudo iptables -C DOCKER-USER -j "$CHAIN" 2>/dev/null; then
  sudo iptables -I DOCKER-USER 1 -j "$CHAIN"
fi

sudo iptables -A "$CHAIN" -m conntrack --ctstate ESTABLISHED,RELATED -j ACCEPT
sudo iptables -A "$CHAIN" -s "$agent_ip" -d "$proxy_ip" -p tcp --dport 3128 -j ACCEPT
sudo iptables -A "$CHAIN" -s "$agent_ip" -j DROP
sudo iptables -A "$CHAIN" -j RETURN

echo "Applied firewall policy in chain $CHAIN"
echo "Shared network: $shared_network"
echo "Allowed: $AGENT_CONTAINER ($agent_ip) -> $PROXY_CONTAINER ($proxy_ip):3128"
