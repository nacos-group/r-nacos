#!/bin/sh

CARGO_BUILD_TARGET="x86_64-unknown-linux-gnu"
if [ "$TARGETPLATFORM" = "linux/arm64" ]; then
  apt-get update && apt-get install -y gcc-10-aarch64-linux-gnu
  CARGO_BUILD_TARGET="aarch64-unknown-linux-gnu"
fi

echo "start build_gnu_docker"
echo "TARGETARCH: " $TARGETARCH
echo "TARGETPLATFORM: " $TARGETPLATFORM
echo "BUILDPLATFORM: " $BUILDPLATFORM
echo "CARGO_BUILD_TARGET: " $CARGO_BUILD_TARGET

rustup target add $CARGO_BUILD_TARGET
cargo build --release --target $CARGO_BUILD_TARGET
mv target/$CARGO_BUILD_TARGET/release/rnacos /usr/bin/rnacos

echo "end build_gnu_docker"
