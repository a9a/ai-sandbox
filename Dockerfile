# syntax=docker/dockerfile:1@sha256:2780b5c3bab67f1f76c781860de469442999ed1a0d7992a5efdf2cffc0e3d769
ARG NODE_IMAGE=node:24-slim@sha256:b506e7321f176aae77317f99d67a24b272c1f09f1d10f1761f2773447d8da26c
FROM ${NODE_IMAGE}

# Pinned version of the agent to ensure build stability
ARG CLAUDE_CODE_VERSION=2.1.109

RUN apt-get update && apt-get install -y --no-install-recommends \
    git \
    curl \
    gosu \
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

WORKDIR /home/devops/project

# Default entrypoint
ENTRYPOINT ["/usr/local/bin/agent-entrypoint.sh"]
