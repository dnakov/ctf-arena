FROM --platform=linux/amd64 ubuntu:24.04 AS builder

RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential \
    ninja-build \
    python3 \
    python3-venv \
    libglib2.0-dev \
    pkg-config \
    curl \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /build
RUN curl -sL https://download.qemu.org/qemu-9.2.0.tar.xz | tar xJ
WORKDIR /build/qemu-9.2.0
RUN ./configure --disable-system --enable-linux-user --enable-plugins \
    --target-list=x86_64-linux-user
RUN make -j$(nproc)
RUN make install DESTDIR=/qemu-install

FROM --platform=linux/amd64 ubuntu:24.04

RUN apt-get update && apt-get install -y --no-install-recommends \
    libglib2.0-0 \
    gcc \
    make \
    pkg-config \
    libglib2.0-dev \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /qemu-install/usr/local/bin/qemu-x86_64 /usr/local/bin/
COPY --from=builder /build/qemu-9.2.0/include/qemu/qemu-plugin.h /plugin/

COPY plugin/ /plugin/
RUN cd /plugin && make

RUN mkdir /work
WORKDIR /work

ENTRYPOINT ["sh", "-c", "qemu-x86_64 -cpu max -plugin /plugin/sandbox.so,limit=$LIMIT /work/binary"]
