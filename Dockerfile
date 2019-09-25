FROM rust:1.37.0-slim AS build

# Missing libaries for build
RUN apt update && apt install -y \
        clang \
        pkg-config \
        libssl-dev

WORKDIR /usr/src
COPY . .

# Setup wasm build env
RUN ./scripts/init.sh

# Build and install binary
# NOTE: Not sure why `install` works but `build --release` doesn't
RUN cargo install --path .
RUN cp target/release/plasma-cash /bin/plasma-cash
RUN rm -rf target

# Libp2p port
EXPOSE 30333
# RPC port
EXPOSE 9944

# TODO Make this into a minimal build using alpine
# NOTE: This requires the following shared libraries:
# linux-vdso.so.1 (0x00007ffd5ff1b000)
# libssl.so.1.1 => /usr/lib/x86_64-linux-gnu/libssl.so.1.1 (0x00007f148202d000)
# libcrypto.so.1.1 => /usr/lib/x86_64-linux-gnu/libcrypto.so.1.1 (0x00007f1481b62000)
# libstdc++.so.6 => /usr/lib/x86_64-linux-gnu/libstdc++.so.6 (0x00007f14817d9000)
# libdl.so.2 => /lib/x86_64-linux-gnu/libdl.so.2 (0x00007f14815d5000)
# librt.so.1 => /lib/x86_64-linux-gnu/librt.so.1 (0x00007f14813cd000)
# libpthread.so.0 => /lib/x86_64-linux-gnu/libpthread.so.0 (0x00007f14811ae000)
# libgcc_s.so.1 => /lib/x86_64-linux-gnu/libgcc_s.so.1 (0x00007f1480f96000)
# libc.so.6 => /lib/x86_64-linux-gnu/libc.so.6 (0x00007f1480ba5000)
# /lib64/ld-linux-x86-64.so.2 (0x00007f148402f000)
# libm.so.6 => /lib/x86_64-linux-gnu/libm.so.6 (0x00007f1480807000)
# (Obtained via `ldd target/release/plasma-cash`)

# Binary
ENTRYPOINT ["plasma-cash"]
