# syntax=docker/dockerfile:1
ARG NODE_IMAGE=node:20-slim@sha256:f93745c153377ee2fbbdd6e24efcd03cd2e86d6ab1d8aa9916a3790c40313a55
FROM ${NODE_IMAGE}

# Pinned version of the agent to ensure build stability
ARG CLAUDE_CODE_VERSION=0.2.9

RUN apt-get update && apt-get install -y --no-install-recommends \
    git \
    curl \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

RUN npm install -g @anthropic-ai/claude-code@${CLAUDE_CODE_VERSION}

# Handle UID 1000 conflict
# The 'node' user already exists with UID 1000 in this image.
# We rename it to 'devops' for clarity.
RUN usermod -l devops node && \
    groupmod -n devops node && \
    usermod -d /home/devops -m devops

COPY scripts/agent-entrypoint.sh /usr/local/bin/agent-entrypoint.sh
RUN chmod +x /usr/local/bin/agent-entrypoint.sh

USER devops
WORKDIR /home/devops/project

# Default entrypoint
ENTRYPOINT ["/usr/local/bin/agent-entrypoint.sh"]
