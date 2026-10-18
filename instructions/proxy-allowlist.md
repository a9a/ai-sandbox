# AI sandbox network access

- Outbound HTTP(S) access is restricted by a Squid domain allowlist.
- When a failed request includes the `X-Squid-Error` response header, treat the error as originating from Squid. If its value starts with `ERR_ACCESS_DENIED`, follow the allowlist guidance below. For any other `X-Squid-Error` value, report the exact Squid error instead of assuming that the domain is missing from the allowlist.
- If a required URL is denied by the proxy, do not bypass the proxy or use a literal IP address.
- Tell the user which domain is required and provide the exact line to add to the allowlist. The entry must contain only the hostname, without a scheme, port, path, or query. Prefer the narrowest required hostname; use a leading dot such as `.example.com` only when access to multiple subdomains is necessary.
- Example: for a blocked URL such as `https://project-docs.github.io/guide`, recommend `project-docs.github.io`. Recommend `.github.io` only when the task requires access to multiple `github.io` subdomains.
- Ask the user to add that line on the host to `~/.config/ai-sandbox/allowed-domains.txt` and run the agent-box launcher with the `squid-reload` subcommand.
- Retry the request after the user confirms that the allowlist was reloaded.
