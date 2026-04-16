FROM debian:13-slim@sha256:4ffb3a1511099754cddc70eb1b12e50ffdb67619aa0ab6c13fcd800a78ef7c7a

RUN apt-get update && apt-get install -y --no-install-recommends \
    squid \
    netcat-openbsd \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Keep squid log directory writable for the proxy user.
RUN mkdir -p /var/log/squid \
    && chown -R proxy:proxy /var/log/squid

COPY squid.conf /etc/squid/squid.conf
COPY allowed-domains.txt /etc/squid/allowed-domains.txt
COPY start-squid.sh /usr/local/bin/start-squid.sh
RUN chmod +x /usr/local/bin/start-squid.sh

EXPOSE 3128

CMD ["/usr/local/bin/start-squid.sh"]
