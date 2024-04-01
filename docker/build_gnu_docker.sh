#!/bin/sh

echo "start build_gnu_docker"
echo $TARGETARCH
echo $CARGO_BUILD_TARGET

if ["$TARGETARCH" = "arm64" ];
then
  rustup target add aarch64-unknown-linux-gnu
  cargo build --release aarch64-unknown-linux-gnu
  mv target/aarch64-unknown-linux-gnu/release/rnacos /usr/bin/rnacos
else
  rustup target add x86_64-unknown-linux-gnu
  cargo build --release x86_64-unknown-linux-gnu
  mv target/x86_64-unknown-linux-gnu/release/rnacos /usr/bin/rnacos
fi

echo "end build_gnu_docker"
