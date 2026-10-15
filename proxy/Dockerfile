FROM debian:13-slim@sha256:a29215f6a35e51e22adffa17f89e9d2ef06214e64a2bad10d765c46aea49f11f

RUN apt-get update && apt-get install -y --no-install-recommends \
    squid \
    haproxy \
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
