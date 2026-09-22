FROM debian:13-slim@sha256:109e2c65005bf160609e4ba6acf7783752f8502ad218e298253428690b9eaa4b

RUN apt-get update && apt-get install -y --no-install-recommends \
    squid \
    netcat-openbsd \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY squid.conf /etc/squid/squid.conf
COPY allowed-domains.txt /etc/squid/allowed-domains.txt

EXPOSE 3128

CMD ["squid", "-N", "-f", "/etc/squid/squid.conf"]
