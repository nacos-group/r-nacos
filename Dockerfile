FROM --platform=$BUILDPLATFORM rust:latest as builder
WORKDIR /usr/src/rnacos
COPY . .
RUN cargo install --path .

FROM debian:bookworm-slim
# RUN apt-get update && apt-get install -y extra-runtime-dependencies && rm -rf /var/lib/apt/lists/*
COPY --from=builder /usr/local/cargo/bin/rnacos /usr/bin/rnacos
ENV USER root
RUN mkdir /io
WORKDIR /io
ENTRYPOINT ["/usr/bin/rnacos"]
