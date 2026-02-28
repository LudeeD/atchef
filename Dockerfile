FROM rust:bookworm AS builder

WORKDIR /build

COPY atproto-api/ atproto-api/
COPY server/ server/

RUN cargo build --release --manifest-path server/Cargo.toml

FROM debian:bookworm-slim AS runtime

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

RUN useradd -r -s /bin/false app \
    && mkdir -p /data \
    && chown app /data

COPY --from=builder /build/server/target/release/atchef /usr/local/bin/atchef

USER app

EXPOSE 3000

VOLUME ["/data"]

ENV DATABASE_PATH=/data/sessions.db \
    BASE_URL=http://localhost:3000

CMD ["/usr/local/bin/atchef"]
