#!/bin/sh
set -eu

mkdir -p /var/log/squid
touch /var/log/squid/access.log /var/log/squid/cache.log
chown proxy:proxy /var/log/squid /var/log/squid/access.log /var/log/squid/cache.log

# Stream squid logs into container logs.
tail -n0 -F /var/log/squid/access.log /var/log/squid/cache.log &
TAIL_PID="$!"

squid -N -f /etc/squid/squid.conf &
SQUID_PID="$!"

wait "$SQUID_PID"
SQUID_STATUS="$?"

kill "$TAIL_PID" 2>/dev/null || true
wait "$TAIL_PID" 2>/dev/null || true

exit "$SQUID_STATUS"
