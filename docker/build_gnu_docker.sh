#!/bin/sh

CARGO_BUILD_TARGET="x86_64-unknown-linux-gnu"
if [ "$TARGETPLATFORM" = "linux/arm64" ]; then
  #CARGO_BUILD_TARGET="aarch64-unknown-linux-gnu"
  #arrch64 暂时统一使用 musl
  CARGO_BUILD_TARGET="aarch64-unknown-linux-musl"
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
