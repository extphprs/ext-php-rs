ARG PHP_VERSION=8.5
ARG TS=-zts

FROM php:${PHP_VERSION}${TS}-alpine

RUN apk add --no-cache \
    llvm17 \
    llvm17-dev \
    llvm17-libs \
    llvm17-static \
    clang17 \
    clang17-dev \
    clang17-static \
    curl

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable
ENV PATH="/root/.cargo/bin:${PATH}"

RUN rustup target add x86_64-unknown-linux-musl
RUN cargo install cargo-expand --locked

ENV PHP=/usr/local/bin/php
ENV PHP_CONFIG=/usr/local/bin/php-config

ENV LLVM_CONFIG_PATH=/usr/lib/llvm17/bin/llvm-config
ENV LIBCLANG_PATH=/usr/lib/llvm17/lib
ENV LD_LIBRARY_PATH=/usr/local/lib:/usr/lib
ENV RUSTFLAGS="-C target-feature=-crt-static -C link-arg=-Wl,-rpath,/usr/local/lib -L /usr/local/lib"

WORKDIR /workspace

COPY . .

ENTRYPOINT ["cargo"]
CMD ["build", "--release", "--no-default-features", "--features", "closure,anyhow,runtime,enum", "--workspace", "--target", "x86_64-unknown-linux-musl"]
