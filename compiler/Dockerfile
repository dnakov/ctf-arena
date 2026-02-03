# Multi-stage build for all-in-one compiler image
# Estimated final size: ~15-20GB

FROM ubuntu:24.04 AS base

ENV DEBIAN_FRONTEND=noninteractive
ENV TZ=UTC

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    curl \
    wget \
    git \
    xz-utils \
    unzip \
    && rm -rf /var/lib/apt/lists/*

# ============================================================
# Stage: C/C++ compilers
# ============================================================
FROM base AS stage-c

RUN apt-get update && apt-get install -y --no-install-recommends \
    gcc \
    g++ \
    musl-dev \
    musl-tools \
    binutils \
    make \
    && rm -rf /var/lib/apt/lists/*

# ============================================================
# Stage: Rust
# ============================================================
FROM base AS stage-rust

ENV RUSTUP_HOME=/usr/local/rustup
ENV CARGO_HOME=/usr/local/cargo
ENV PATH=/usr/local/cargo/bin:$PATH

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --no-modify-path \
    && rustup target add x86_64-unknown-linux-musl

# ============================================================
# Stage: Go
# ============================================================
FROM base AS stage-go

ENV GOROOT=/usr/local/go
ENV PATH=/usr/local/go/bin:$PATH

RUN curl -fsSL https://go.dev/dl/go1.23.5.linux-amd64.tar.gz | tar -C /usr/local -xzf -

# ============================================================
# Stage: Zig
# ============================================================
FROM base AS stage-zig

RUN curl -fsSL https://ziglang.org/download/0.13.0/zig-linux-x86_64-0.13.0.tar.xz | tar -C /usr/local -xJf - \
    && ln -s /usr/local/zig-linux-x86_64-0.13.0 /usr/local/zig

# ============================================================
# Stage: GraalVM + native-image (for JVM languages)
# ============================================================
FROM base AS stage-graalvm

ENV JAVA_HOME=/usr/local/graalvm
ENV PATH=/usr/local/graalvm/bin:$PATH

# GraalVM 21+ includes native-image by default (no gu needed)
RUN curl -fsSL https://github.com/graalvm/graalvm-ce-builds/releases/download/jdk-21.0.2/graalvm-community-jdk-21.0.2_linux-x64_bin.tar.gz | tar -C /usr/local -xzf - \
    && mv /usr/local/graalvm-community-openjdk-21.0.2+13.1 /usr/local/graalvm

# Install Kotlin compiler
RUN curl -fsSL https://github.com/JetBrains/kotlin/releases/download/v2.0.0/kotlin-compiler-2.0.0.zip -o /tmp/kotlin.zip \
    && unzip /tmp/kotlin.zip -d /usr/local \
    && rm /tmp/kotlin.zip

# Install Scala
RUN curl -fsSL https://github.com/scala/scala3/releases/download/3.4.2/scala3-3.4.2.tar.gz | tar -C /usr/local -xzf -

# Install Clojure
RUN curl -fsSL https://download.clojure.org/install/linux-install-1.11.3.1463.sh | bash

# ============================================================
# Stage: .NET SDK (for C# AOT)
# ============================================================
FROM base AS stage-dotnet

ENV DOTNET_ROOT=/usr/local/dotnet
ENV PATH=/usr/local/dotnet:$PATH

RUN curl -fsSL https://dot.net/v1/dotnet-install.sh | bash -s -- --channel 8.0 --install-dir /usr/local/dotnet

# ============================================================
# Stage: Swift
# ============================================================
FROM base AS stage-swift

RUN apt-get update && apt-get install -y --no-install-recommends \
    libc6-dev \
    libcurl4-openssl-dev \
    libedit-dev \
    libpython3-dev \
    libsqlite3-dev \
    libxml2-dev \
    pkg-config \
    && rm -rf /var/lib/apt/lists/*

RUN curl -fsSL https://download.swift.org/swift-5.10.1-release/ubuntu2404/swift-5.10.1-RELEASE/swift-5.10.1-RELEASE-ubuntu24.04.tar.gz | tar -C /usr/local -xzf - \
    && ln -s /usr/local/swift-5.10.1-RELEASE-ubuntu24.04 /usr/local/swift

# ============================================================
# Stage: Haskell (GHC)
# ============================================================
FROM base AS stage-haskell

RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential \
    libgmp-dev \
    libc6-dev \
    libffi-dev \
    libncurses-dev \
    libtinfo-dev \
    && rm -rf /var/lib/apt/lists/*

RUN curl --proto '=https' --tlsv1.2 -sSf https://get-ghcup.haskell.org | BOOTSTRAP_HASKELL_NONINTERACTIVE=1 sh

# Install common Haskell packages for networking
ENV PATH="/root/.ghcup/bin:$PATH"
RUN cabal update && cabal install --lib network

# ============================================================
# Stage: Other native compilers
# ============================================================
FROM base AS stage-native

RUN apt-get update && apt-get install -y --no-install-recommends \
    # Nim
    nim \
    # Pascal
    fpc \
    # OCaml
    ocaml \
    ocaml-findlib \
    # Lua
    lua5.4 \
    luac5.4 \
    liblua5.4-dev \
    # Perl PAR::Packer
    perl \
    libpar-packer-perl \
    # Erlang/Elixir
    erlang \
    elixir \
    # Racket
    racket \
    # WASM tools
    wabt \
    && rm -rf /var/lib/apt/lists/*

# ============================================================
# Stage: Python with Nuitka
# ============================================================
FROM base AS stage-python

RUN apt-get update && apt-get install -y --no-install-recommends \
    python3 \
    python3-pip \
    python3-dev \
    patchelf \
    ccache \
    && rm -rf /var/lib/apt/lists/*

RUN pip3 install --break-system-packages nuitka ordered-set zstandard

# ============================================================
# Stage: Bun (for TypeScript/JavaScript)
# ============================================================
FROM base AS stage-bun

RUN curl -fsSL https://bun.sh/install | bash
ENV BUN_INSTALL=/root/.bun
ENV PATH=/root/.bun/bin:$PATH

# ============================================================
# Stage: Deno
# ============================================================
FROM base AS stage-deno

RUN curl -fsSL https://deno.land/install.sh | sh
ENV DENO_INSTALL=/root/.deno
ENV PATH=/root/.deno/bin:$PATH

# Pre-cache the deno runtime for compile (creates denort cache)
RUN echo 'console.log("hello")' > /tmp/test.ts && \
    /root/.deno/bin/deno compile --output /tmp/testbin /tmp/test.ts && \
    rm -f /tmp/test.ts /tmp/testbin

# ============================================================
# Stage: Node.js (for pkg bundling)
# ============================================================
FROM base AS stage-node

RUN curl -fsSL https://deb.nodesource.com/setup_22.x | bash - \
    && apt-get install -y nodejs \
    && npm install -g @yao-pkg/pkg

# Pre-fetch the Node.js binary for pkg (caches to ~/.pkg-cache)
RUN echo 'console.log("hello")' > /tmp/test.js && \
    pkg /tmp/test.js --target node22-linux-x64 --output /tmp/testbin && \
    rm -f /tmp/test.js /tmp/testbin

# ============================================================
# Final: Combine everything
# ============================================================
FROM ubuntu:24.04

ENV DEBIAN_FRONTEND=noninteractive
ENV TZ=UTC

# Base tools
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    curl \
    wget \
    git \
    xz-utils \
    unzip \
    # C/C++
    gcc \
    g++ \
    musl-dev \
    musl-tools \
    binutils \
    make \
    # For various builds
    pkg-config \
    libgmp-dev \
    libc6-dev \
    zlib1g-dev \
    # Utilities
    xxd \
    # Nim
    nim \
    # Pascal
    fpc \
    # OCaml
    ocaml \
    # Lua (with static library)
    lua5.4 \
    liblua5.4-0 \
    liblua5.4-dev \
    # Perl
    perl \
    libpar-packer-perl \
    # Erlang/Elixir
    erlang \
    elixir \
    # Racket
    racket \
    # WASM tools
    wabt \
    # Python
    python3 \
    python3-pip \
    python3-dev \
    patchelf \
    ccache \
    && rm -rf /var/lib/apt/lists/*

# Python packages
RUN pip3 install --break-system-packages nuitka ordered-set zstandard

# Rust
ENV RUSTUP_HOME=/usr/local/rustup
ENV CARGO_HOME=/usr/local/cargo
COPY --from=stage-rust /usr/local/rustup /usr/local/rustup
COPY --from=stage-rust /usr/local/cargo /usr/local/cargo

# Go
COPY --from=stage-go /usr/local/go /usr/local/go

# Zig
COPY --from=stage-zig /usr/local/zig-linux-x86_64-0.13.0 /usr/local/zig

# GraalVM + JVM tools
COPY --from=stage-graalvm /usr/local/graalvm /usr/local/graalvm
COPY --from=stage-graalvm /usr/local/kotlinc /usr/local/kotlinc
COPY --from=stage-graalvm /usr/local/scala3-3.4.2 /usr/local/scala
COPY --from=stage-graalvm /usr/local/bin/clojure /usr/local/bin/clojure
COPY --from=stage-graalvm /usr/local/bin/clj /usr/local/bin/clj
COPY --from=stage-graalvm /usr/local/lib/clojure /usr/local/lib/clojure

# .NET
COPY --from=stage-dotnet /usr/local/dotnet /usr/local/dotnet

# Swift
COPY --from=stage-swift /usr/local/swift-5.10.1-RELEASE-ubuntu24.04 /usr/local/swift

# Haskell (GHC + cabal packages)
COPY --from=stage-haskell /root/.ghcup /root/.ghcup
COPY --from=stage-haskell /root/.cabal /root/.cabal

# Bun
COPY --from=stage-bun /root/.bun /root/.bun

# Deno (includes pre-cached runtime)
COPY --from=stage-deno /root/.deno /root/.deno
COPY --from=stage-deno /root/.cache /root/.cache

# Node.js + pkg (includes pre-cached Node.js binary)
COPY --from=stage-node /usr/bin/node /usr/bin/node
COPY --from=stage-node /usr/bin/npm /usr/bin/npm
COPY --from=stage-node /usr/bin/npx /usr/bin/npx
COPY --from=stage-node /usr/lib/node_modules /usr/lib/node_modules
COPY --from=stage-node /root/.pkg-cache /root/.pkg-cache

# Set up PATH
ENV PATH=/usr/local/cargo/bin:/usr/local/go/bin:/usr/local/zig:/usr/local/graalvm/bin:/usr/local/kotlinc/bin:/usr/local/scala/bin:/usr/local/dotnet:/usr/local/swift/usr/bin:/root/.ghcup/bin:/root/.bun/bin:/root/.deno/bin:$PATH
ENV JAVA_HOME=/usr/local/graalvm
ENV DOTNET_ROOT=/usr/local/dotnet
ENV GOROOT=/usr/local/go
ENV BUN_INSTALL=/root/.bun
ENV DENO_INSTALL=/root/.deno

# Copy compile scripts
COPY compile.sh /compiler/compile.sh
COPY scripts/ /compiler/scripts/

RUN chmod +x /compiler/compile.sh /compiler/scripts/*.sh

WORKDIR /work

ENTRYPOINT ["/compiler/compile.sh"]
