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

ENV PATH /root/.cargo/bin:$PATH

ADD . /rnacos/

# Manually update the timestamps as ADD keeps the local timestamps and cargo would then believe the cache is fresh
RUN touch /rnacos/src/lib.rs /rnacos/src/main.rs

RUN cargo build --release \
    && mv /rnacos/target/release/rnacos /usr/bin/rnacos

FROM base-$TARGETARCH

ENV PATH /root/.cargo/bin:$PATH
ENV USER root

RUN mkdir /io

COPY --from=builder /usr/bin/rnacos /usr/bin/rnacos

WORKDIR /io

ENTRYPOINT ["/usr/bin/rnacos"]
