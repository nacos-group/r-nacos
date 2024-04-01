#!/bin/sh

echo "start build_gnu_docker"
echo "TARGETARCH: " $TARGETARCH
echo "TARGETPLATFORM: " $TARGETPLATFORM
echo "BUILDPLATFORM: " $BUILDPLATFORM

CARGO_BUILD_TARGET="x86_64-unknown-linux-gnu"
if [ "$TARGETPLATFORM" = "linux/arm64" ]; then
  CARGO_BUILD_TARGET="aarch64-unknown-linux-gnu"
fi

echo "CARGO_BUILD_TARGET: " $CARGO_BUILD_TARGET

if [ "$TARGETPLATFORM" = "linux/arm64" ];
then
  echo "build aarch64-unknown-linux-gnu" $TARGETPLATFORM
  rustup target add aarch64-unknown-linux-gnu
  cargo build --release --target aarch64-unknown-linux-gnu
  mv target/aarch64-unknown-linux-gnu/release/rnacos /usr/bin/rnacos
else
  echo "build  x86_64-unknown-linux-gnu" $TARGETPLATFORM
  rustup target add x86_64-unknown-linux-gnu
  cargo build --release --target x86_64-unknown-linux-gnu
  mv target/x86_64-unknown-linux-gnu/release/rnacos /usr/bin/rnacos
fi

echo "end build_gnu_docker"
