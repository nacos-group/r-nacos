# x86_64 base
FROM quay.io/pypa/manylinux2014_x86_64 as base-amd64
# x86_64 builder
FROM --platform=$BUILDPLATFORM messense/rust-musl-cross:x86_64-musl as builder-amd64

# aarch64 base
FROM quay.io/pypa/manylinux2014_aarch64 as base-arm64
# aarch64 cross compile builder
FROM --platform=$BUILDPLATFORM messense/rust-musl-cross:aarch64-musl as builder-arm64

ARG TARGETARCH
FROM builder-$TARGETARCH as builder

ENV USER root
ENV PATH /root/.cargo/bin:$PATH

# Compile dependencies only for build caching
ADD Cargo.toml /rnacos/Cargo.toml
RUN cd /rnacos && \ 
    --mount=type=cache,target=/root/.cargo/git \
    --mount=type=cache,target=/root/.cargo/registry \
    --mount=type=cache,target=/rnacos/target,sharing=locked \
    mkdir /rnacos/src && \
    touch  /rnacos/src/lib.rs && \
    echo 'fn main() { println!("Dummy") }' > /rnacos/src/main.rs && \
    cargo rustc --target $CARGO_BUILD_TARGET --bin rnacos --manifest-path /rnacos/Cargo.toml --release --features password-storage -- -C link-arg=-s

ADD . /rnacos/

# Manually update the timestamps as ADD keeps the local timestamps and cargo would then believe the cache is fresh
RUN touch /rnacos/src/lib.rs /rnacos/src/main.rs

RUN cd /rnacos && \ 
    --mount=type=cache,target=/root/.cargo/git \
    --mount=type=cache,target=/root/.cargo/registry \
    --mount=type=cache,target=/rnacos/target,sharing=locked \
    cargo rustc --target $CARGO_BUILD_TARGET --bin rnacos --manifest-path /rnacos/Cargo.toml --release --features password-storage -- -C link-arg=-s \
    && mv /rnacos/target/$CARGO_BUILD_TARGET/release/rnacos /usr/bin/rnacos

FROM base-$TARGETARCH

ENV PATH /root/.cargo/bin:$PATH
ENV USER root

RUN mkdir /io

COPY --from=builder /usr/bin/rnacos /usr/bin/rnacos

WORKDIR /io

ENTRYPOINT ["/usr/bin/rnacos"]
