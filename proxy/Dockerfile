FROM debian:12-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    squid \
    netcat-openbsd \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY squid.conf /etc/squid/squid.conf
COPY allowed-domains.txt /etc/squid/allowed-domains.txt

EXPOSE 3128

CMD ["squid", "-N", "-f", "/etc/squid/squid.conf"]
