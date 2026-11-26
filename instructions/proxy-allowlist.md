# AI sandbox network access

- Outbound HTTP(S) access is restricted by a Squid domain allowlist.
- When an inner Docker container needs outbound HTTP(S), use the Docker daemon's network namespace and pass the proxy settings explicitly: `docker run --network host -e HTTP_PROXY=http://proxy:3128 -e HTTPS_PROXY=http://proxy:3128 -e NO_PROXY=localhost,127.0.0.1,proxy -e http_proxy=http://proxy:3128 -e https_proxy=http://proxy:3128 -e no_proxy=localhost,127.0.0.1,proxy IMAGE`. Add any local services that need direct access to `NO_PROXY` and `no_proxy`. The default inner Docker network may not be able to resolve or reach `proxy`.
- Host networking here shares the inner Docker daemon's network namespace, including its ports; check for port conflicts and use it only when compatible with the workload. For Docker builds needing HTTP(S), use host networking and pass the proxy settings as build arguments when the builder supports it; verify the builder's network and proxy behavior. Proxy settings in the agent or daemon are not necessarily inherited by containers or builds.
- Do not bypass the proxy to reach external destinations.
- When a failed request includes the `X-Squid-Error` response header, treat the error as originating from Squid. If its value starts with `ERR_ACCESS_DENIED`, follow the allowlist guidance below. For any other `X-Squid-Error` value, report the exact Squid error instead of assuming that the domain is missing from the allowlist.
- If a required URL is denied by the proxy, do not bypass the proxy or use a literal IP address.
- Tell the user which domain is required and provide the exact line to add to the allowlist. The entry must contain only the hostname, without a scheme, port, path, or query. Prefer the narrowest required hostname; use a leading dot such as `.example.com` only when access to multiple subdomains is necessary.
- Example: for a blocked URL such as `https://project-docs.github.io/guide`, recommend `project-docs.github.io`. Recommend `.github.io` only when the task requires access to multiple `github.io` subdomains.
- Ask the user to add that line on the host to `~/.config/ai-sandbox/allowed-domains.txt` and run the agent-box launcher with the `squid-reload` subcommand.
- Retry the request after the user confirms that the allowlist was reloaded.
