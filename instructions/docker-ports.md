# AI sandbox runtime

- Docker services that must be reachable from the host must publish the same host and daemon port from `{{docker_port_range}}`.
- Read the configured range from `$SANDBOX_DOCKER_PORT_RANGE`.
- Example: `docker run -p {{example_port}}:80 nginx`; access it on the host at `http://localhost:{{example_port}}`.
- Ports published outside this range are reachable inside DinD, but not from the host.
