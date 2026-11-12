# Build from the repo root: docker build -f crates/server/Dockerfile -t sabi-server .
FROM rust:1-bookworm AS build
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
# The workspace release profile is tuned for a tiny WASM file (size-optimized,
# full LTO, one codegen unit). For the server, build for speed instead: it
# compiles much faster and in less memory on small build machines.
ENV CARGO_PROFILE_RELEASE_OPT_LEVEL=3 \
    CARGO_PROFILE_RELEASE_LTO=thin \
    CARGO_PROFILE_RELEASE_CODEGEN_UNITS=16 \
    CARGO_PROFILE_RELEASE_PANIC=unwind
RUN cargo build --release --locked -p sabi-server

FROM debian:bookworm-slim
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --system --no-create-home sabi
COPY --from=build /app/target/release/sabi-server /usr/local/bin/sabi-server
USER sabi
# Hosts like Render set PORT themselves; this is the fallback.
ENV PORT=8080
EXPOSE 8080
CMD ["sabi-server"]
