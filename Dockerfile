# syntax=docker/dockerfile:1
ARG NODE_IMAGE=node:20-slim@sha256:87ef9545464152504958f33887010424a106f0f29c4202353e6b206981f3d81b
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

USER devops
WORKDIR /home/devops/project

# Default entrypoint
ENTRYPOINT ["claude"]
