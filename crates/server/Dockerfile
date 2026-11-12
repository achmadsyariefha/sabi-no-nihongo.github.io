# Build from the repo root: docker build -f crates/server/Dockerfile -t sabi-server .
FROM rust:1-bookworm AS build
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
RUN cargo build --release --locked -p sabi-server

FROM debian:bookworm-slim
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --system --no-create-home sabi
COPY --from=build /app/target/release/sabi-server /usr/local/bin/sabi-server
USER sabi
ENV PORT=8080
EXPOSE 8080
CMD ["sabi-server"]
