FROM debian:13-slim@sha256:4ffb3a1511099754cddc70eb1b12e50ffdb67619aa0ab6c13fcd800a78ef7c7a

RUN apt-get update && apt-get install -y --no-install-recommends \
    squid \
    netcat-openbsd \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY squid.conf /etc/squid/squid.conf
COPY allowed-domains.txt /etc/squid/allowed-domains.txt

EXPOSE 3128

CMD ["squid", "-N", "-f", "/etc/squid/squid.conf"]
