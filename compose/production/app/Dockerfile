FROM rust:1.99.1-bookworm AS build
WORKDIR /source
COPY . .
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/source/target \
    cargo build --locked --release --workspace --bins \
    && install -d /artifacts \
    && install target/release/api target/release/worker target/release/maintenance target/release/manage target/release/migration /artifacts/

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --uid 10001 --create-home app
COPY --from=build /artifacts/ /usr/local/bin/
USER 10001:10001
WORKDIR /home/app
ENV BIND_ADDRESS=0.0.0.0:3000
EXPOSE 3000
ENTRYPOINT ["api"]
