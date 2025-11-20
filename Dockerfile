FROM rust:latest AS base
ARG PHP_VERSION=8.5
WORKDIR /tmp
RUN <<EOF
set -e

apt update -y
apt install -y \
  libclang-dev \
  bison \
  re2c \
  curl \
  jq

# Download and extract PHP
FULL_VERSION=$(curl -fsSL "https://www.php.net/releases/index.php?json&version=${PHP_VERSION}" | jq -r '.version')
echo "Downloading PHP ${FULL_VERSION}..."
curl -fsSL "https://www.php.net/distributions/php-${FULL_VERSION}.tar.gz" -o php.tar.gz
tar -xzf php.tar.gz
rm php.tar.gz
mv "php-${FULL_VERSION}" php-src

# Build PHP
cd php-src
./configure \
    --enable-debug \
    --disable-all --disable-cgi
make -j "$(nproc)"
make install
EOF

FROM base AS docsrs_bindings_builder
WORKDIR /src
RUN rustup component add rustfmt
RUN --mount=type=bind,target=/src,rw <<EOF
set -e
cargo clean
cargo build
cp target/debug/build/ext-php-rs-*/out/bindings.rs /docsrs_bindings.rs
rustfmt /docsrs_bindings.rs
EOF
ENTRYPOINT ["/generate.sh"]

FROM scratch AS docsrs_bindings
COPY --from=docsrs_bindings_builder /docsrs_bindings.rs /
