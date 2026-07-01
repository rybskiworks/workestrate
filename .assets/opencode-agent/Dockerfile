## =============================================================================
## Agent Runtime - Flat & Unfucked
## =============================================================================
#FROM node:24-bookworm-slim
#
#ARG CONTAINER_USER_HOME=/home/node
#ARG CONTAINER_USER=node
## CACHE BUSTER: Change this ARG at build time to force an update of opencode-ai
#ARG OPENCODE_VERSION=latest
#
## 1. Install System Dependencies (xz-utils, unzip, and python3-pip included)
#RUN apt-get update && apt-get install -y --no-install-recommends \
#    ca-certificates \
#    curl \
#    wget \
#    gnupg \
#    git \
#    gettext-base \
#    docker.io \
#    docker-compose \
#    python3 \
#    python3-pip \
#    jq \
#    lynx \
#    chromium \
#    xz-utils \
#    unzip \
#    && mkdir -p -m 755 /etc/apt/keyrings \
#    && mkdir -p -m 755 /etc/apt/sources.list.d \
#    \
#    && out="$(mktemp)" \
#    && wget -nv -O "$out" https://cli.github.com/packages/githubcli-archive-keyring.gpg \
#    && cat "$out" > /etc/apt/keyrings/githubcli-archive-keyring.gpg \
#    && chmod go+r /etc/apt/keyrings/githubcli-archive-keyring.gpg \
#    && echo "deb [arch=$(dpkg --print-architecture) signed-by=/etc/apt/keyrings/githubcli-archive-keyring.gpg] https://cli.github.com/packages stable main" \
#        > /etc/apt/sources.list.d/github-cli.list \
#    \
#    && wget -nv -O- https://acli.atlassian.com/gpg/public-key.asc \
#        | gpg --dearmor -o /etc/apt/keyrings/acli-archive-keyring.gpg \
#    && chmod go+r /etc/apt/keyrings/acli-archive-keyring.gpg \
#    && echo "deb [arch=$(dpkg --print-architecture) signed-by=/etc/apt/keyrings/acli-archive-keyring.gpg] https://acli.atlassian.com/linux/deb stable main" \
#        > /etc/apt/sources.list.d/acli.list \
#    \
#    && apt-get update \
#    && apt-get install -y --no-install-recommends \
#        gh \
#        acli \
#    && rm -rf /var/lib/apt/lists/* "$out"
#
## 2. Create nix directory structure as root but prepare for node user
#RUN mkdir -p /nix \
#    && chown -R ${CONTAINER_USER}:${CONTAINER_USER} /nix
#
## Switch to standard user
#USER ${CONTAINER_USER}
#
### 3. Install Package Managers & Runtimes (Nix, Bun, uv)
##RUN curl -L https://nixos.org/nix/install | sh -s -- --no-daemon \
##    && curl -fsSL https://bun.sh/install | bash \
##    && curl -LsSf https://astral.sh/uv/install.sh | sh
#
## 3. Install Package Managers & Runtimes (Nix, Bun, uv)
#RUN curl -L https://nixos.org/nix/install | sh -s -- --no-daemon --yes \
#    && . ${CONTAINER_USER_HOME}/.nix-profile/etc/profile.d/nix.sh \
#    && nix --version \
#    && curl -fsSL https://bun.sh/install | bash \
#    && curl -LsSf https://astral.sh/uv/install.sh | sh
#
#RUN mkdir -p ${CONTAINER_USER_HOME}/.config/nix \
#    && printf '%s\n' \
#        'experimental-features = nix-command flakes' \
#        'accept-flake-config = true' \
#        'sandbox = false' \
#        > ${CONTAINER_USER_HOME}/.config/nix/nix.conf
#
## 3a. Install CLI Tools (User context) - Tavily removed from here
#RUN ~/.bun/bin/bun add -g @sentry/cli \
#    && ~/.local/bin/uv tool install specify-cli --from git+https://github.com/github/spec-kit.git \
#    && ~/.local/bin/uv tool install mcp-server-sqlite
#
## Switch back to root for global installs
#USER root
#
## 4. Install Global NPM Tools & Python Libraries (Tavily installed globally here)
#RUN pip3 install --no-cache-dir --break-system-packages tavily-python \
#    && npm install -g \
#    opencode-ai@${OPENCODE_VERSION} \
#    @modelcontextprotocol/server-filesystem \
#    @modelcontextprotocol/server-memory \
#    mcporter \
#    contentful-cli
##    playwright-cli
#
## 5. Setup Filesystem
#RUN mkdir -p ${CONTAINER_USER_HOME}/.local/state \
#    && mkdir -p ${CONTAINER_USER_HOME}/.local/share/opencode \
#    && mkdir -p ${CONTAINER_USER_HOME}/.opencode \
#    && mkdir -p ${CONTAINER_USER_HOME}/.config/opencode/skills \
#    && mkdir -p ${CONTAINER_USER_HOME}/.config/opencode/plugins \
#    && mkdir -p ${CONTAINER_USER_HOME}/memory \
#    && mkdir -p ${CONTAINER_USER_HOME}/projects \
#    && mkdir -p ${CONTAINER_USER_HOME}/workbench \
#    && mkdir -p ${CONTAINER_USER_HOME}/worktrees \
#    && mkdir -p ${CONTAINER_USER_HOME}/outputs \
#    && mkdir -p ${CONTAINER_USER_HOME}/workspace/frontend \
#    && mkdir -p ${CONTAINER_USER_HOME}/workspace/backend \
#    && chown -R ${CONTAINER_USER}:${CONTAINER_USER} ${CONTAINER_USER_HOME}
#
## 6. Install MCP Server dependencies
#COPY --chown=${CONTAINER_USER}:${CONTAINER_USER} package-mcp.json ${CONTAINER_USER_HOME}/.config/opencode/package.json
#RUN cd ${CONTAINER_USER_HOME}/.config/opencode && npm install
#
## 7. Copy MCP server script
#COPY --chown=${CONTAINER_USER}:${CONTAINER_USER} mcp-server.js ${CONTAINER_USER_HOME}/.config/opencode/mcp-server.js
#
## 8. Final Config
#COPY entrypoint.sh /usr/local/bin/entrypoint.sh
#RUN chmod 755 /usr/local/bin/entrypoint.sh
#
#USER ${CONTAINER_USER}
#WORKDIR ${CONTAINER_USER_HOME}
#
#ENV NODE_ENV=production
#ENV SERVER_MODE=mcp
##ENV PATH="${CONTAINER_USER_HOME}/.local/bin:${CONTAINER_USER_HOME}/.bun/bin:${CONTAINER_USER_HOME}/.nix-profile/bin:${CONTAINER_USER_HOME}/.nix-profile/sbin:${PATH}"
##ENV NIX_PATH="nixpkgs=https://github.com/NixOS/nixpkgs/archive/nixos-unstable.tar.gz"
##
##EXPOSE 3000 8080
##ENTRYPOINT ["/usr/local/bin/entrypoint.sh"]
#
#ENV HOME=${CONTAINER_USER_HOME}
#ENV USER=${CONTAINER_USER}
#ENV NODE_ENV=production
#ENV SERVER_MODE=mcp
#ENV PATH="${CONTAINER_USER_HOME}/.local/bin:${CONTAINER_USER_HOME}/.bun/bin:${CONTAINER_USER_HOME}/.nix-profile/bin:${CONTAINER_USER_HOME}/.nix-profile/sbin:${PATH}"
#ENV NIX_PATH="nixpkgs=https://github.com/NixOS/nixpkgs/archive/nixos-unstable.tar.gz"
#
#EXPOSE 3000 8080
#ENTRYPOINT ["/usr/local/bin/entrypoint.sh"]

# =============================================================================
# Agent Runtime - Flat & Unfucked
# =============================================================================
FROM node:24-bookworm-slim

ARG CONTAINER_USER_HOME=/home/node
ARG CONTAINER_USER=node
# CACHE BUSTER: Change this ARG at build time to force an update of opencode-ai
ARG OPENCODE_VERSION=latest

# 1. Install System Dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    curl \
    wget \
    gnupg \
    git \
    gettext-base \
    docker.io \
    docker-compose \
    python3 \
    python3-pip \
    jq \
    lynx \
    chromium \
    xz-utils \
    unzip \
    bash \
    && mkdir -p -m 755 /etc/apt/keyrings \
    && mkdir -p -m 755 /etc/apt/sources.list.d \
    \
    && out="$(mktemp)" \
    && wget -nv -O "$out" https://cli.github.com/packages/githubcli-archive-keyring.gpg \
    && cat "$out" > /etc/apt/keyrings/githubcli-archive-keyring.gpg \
    && chmod go+r /etc/apt/keyrings/githubcli-archive-keyring.gpg \
    && echo "deb [arch=$(dpkg --print-architecture) signed-by=/etc/apt/keyrings/githubcli-archive-keyring.gpg] https://cli.github.com/packages stable main" \
        > /etc/apt/sources.list.d/github-cli.list \
    \
    && wget -nv -O- https://acli.atlassian.com/gpg/public-key.asc \
        | gpg --dearmor -o /etc/apt/keyrings/acli-archive-keyring.gpg \
    && chmod go+r /etc/apt/keyrings/acli-archive-keyring.gpg \
    && echo "deb [arch=$(dpkg --print-architecture) signed-by=/etc/apt/keyrings/acli-archive-keyring.gpg] https://acli.atlassian.com/linux/deb stable main" \
        > /etc/apt/sources.list.d/acli.list \
    \
    && apt-get update \
    && apt-get install -y --no-install-recommends \
        gh \
        acli \
    && rm -rf /var/lib/apt/lists/* "$out"

# 2. Prepare /nix for single-user Nix install
RUN mkdir -p /nix \
    && chown -R ${CONTAINER_USER}:${CONTAINER_USER} /nix

# 3. Install Nix, Bun, uv as node user
USER ${CONTAINER_USER}
WORKDIR ${CONTAINER_USER_HOME}

ENV HOME=${CONTAINER_USER_HOME}
ENV USER=${CONTAINER_USER}

RUN curl -L https://nixos.org/nix/install | sh -s -- --no-daemon --yes \
    && export PATH="${CONTAINER_USER_HOME}/.nix-profile/bin:${CONTAINER_USER_HOME}/.nix-profile/sbin:$PATH" \
    && ${CONTAINER_USER_HOME}/.nix-profile/bin/nix --version \
    && curl -fsSL https://bun.sh/install | bash \
    && curl -LsSf https://astral.sh/uv/install.sh | sh

# 3b. Configure Nix
RUN mkdir -p ${CONTAINER_USER_HOME}/.config/nix \
    && printf '%s\n' \
        'experimental-features = nix-command flakes' \
        'accept-flake-config = true' \
        'sandbox = false' \
        > ${CONTAINER_USER_HOME}/.config/nix/nix.conf

# 3c. Install user-level CLI tools
RUN ${CONTAINER_USER_HOME}/.bun/bin/bun add -g @sentry/cli \
    && ${CONTAINER_USER_HOME}/.local/bin/uv tool install specify-cli --from git+https://github.com/github/spec-kit.git \
    && ${CONTAINER_USER_HOME}/.local/bin/uv tool install mcp-server-sqlite

# 4. Install root-level global tools
USER root

RUN pip3 install --no-cache-dir --break-system-packages tavily-python \
    && npm install -g \
        opencode-ai@${OPENCODE_VERSION} \
        @modelcontextprotocol/server-filesystem \
        @modelcontextprotocol/server-memory \
        mcporter \
        contentful-cli

# 5. Setup filesystem
RUN mkdir -p ${CONTAINER_USER_HOME}/.local/state \
    && mkdir -p ${CONTAINER_USER_HOME}/.local/share/opencode \
    && mkdir -p ${CONTAINER_USER_HOME}/.opencode \
    && mkdir -p ${CONTAINER_USER_HOME}/.config/opencode/skills \
    && mkdir -p ${CONTAINER_USER_HOME}/.config/opencode/plugins \
    && mkdir -p ${CONTAINER_USER_HOME}/memory \
    && mkdir -p ${CONTAINER_USER_HOME}/projects \
    && mkdir -p ${CONTAINER_USER_HOME}/workbench \
    && mkdir -p ${CONTAINER_USER_HOME}/worktrees \
    && mkdir -p ${CONTAINER_USER_HOME}/outputs \
    && mkdir -p ${CONTAINER_USER_HOME}/workspace/frontend \
    && mkdir -p ${CONTAINER_USER_HOME}/workspace/backend \
    && chown -R ${CONTAINER_USER}:${CONTAINER_USER} ${CONTAINER_USER_HOME}

# 6. Install MCP server dependencies as node user
COPY --chown=${CONTAINER_USER}:${CONTAINER_USER} package-mcp.json ${CONTAINER_USER_HOME}/.config/opencode/package.json

USER ${CONTAINER_USER}
RUN cd ${CONTAINER_USER_HOME}/.config/opencode && npm install

# 7. Copy MCP server script and entrypoint
USER root

COPY --chown=${CONTAINER_USER}:${CONTAINER_USER} mcp-server.js ${CONTAINER_USER_HOME}/.config/opencode/mcp-server.js
COPY entrypoint.sh /usr/local/bin/entrypoint.sh

RUN chmod 755 /usr/local/bin/entrypoint.sh \
    && chown -R ${CONTAINER_USER}:${CONTAINER_USER} ${CONTAINER_USER_HOME}

# 8. Final runtime config
USER ${CONTAINER_USER}
WORKDIR ${CONTAINER_USER_HOME}

ENV HOME=${CONTAINER_USER_HOME}
ENV USER=${CONTAINER_USER}
ENV NODE_ENV=production
ENV SERVER_MODE=mcp
ENV PATH="${CONTAINER_USER_HOME}/.local/bin:${CONTAINER_USER_HOME}/.bun/bin:${CONTAINER_USER_HOME}/.nix-profile/bin:${CONTAINER_USER_HOME}/.nix-profile/sbin:${PATH}"
ENV NIX_PATH="nixpkgs=https://github.com/NixOS/nixpkgs/archive/nixos-unstable.tar.gz"

EXPOSE 3000 8080

ENTRYPOINT ["/usr/local/bin/entrypoint.sh"]