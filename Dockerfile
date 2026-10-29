# syntax=docker/dockerfile:1

# Builder: cross-compile the server to a static musl binary and build the WASM
# client. A Debian base is used on purpose: cargo-leptos downloads wasm-bindgen
# and wasm-opt as glibc binaries, which do not run on a musl host. The server
# itself still targets musl so the runtime image can stay minimal.
FROM rust:1.97-bookworm AS builder

RUN apt-get update \
    && apt-get install -y --no-install-recommends musl-tools \
    && rm -rf /var/lib/apt/lists/*

RUN rustup target add x86_64-unknown-linux-musl wasm32-unknown-unknown
RUN cargo install cargo-leptos --locked --version 0.3.7

WORKDIR /app
COPY . .

# Release build: static musl server binary plus hashed site assets under
# target/site. LEPTOS_BIN_TARGET_TRIPLE tells cargo-leptos which target the
# server binary uses (see scripts/build-release.sh).
RUN LEPTOS_BIN_TARGET_TRIPLE=x86_64-unknown-linux-musl cargo leptos build --release

# cargo-leptos renames webrpg_bg.wasm to webrpg.wasm without patching the
# hydration script that still asks for the _bg name, so ship both.
RUN if [ -f target/site/pkg/webrpg.wasm ] && [ ! -f target/site/pkg/webrpg_bg.wasm ]; then \
        cp target/site/pkg/webrpg.wasm target/site/pkg/webrpg_bg.wasm; \
    fi

# Runtime: the static binary needs almost nothing. Alpine keeps a shell for
# debugging; ca-certificates covers any outbound TLS.
FROM alpine:3.21 AS runtime

RUN apk add --no-cache ca-certificates \
    && addgroup -S webrpg \
    && adduser -S -G webrpg webrpg

WORKDIR /app
COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/webrpg ./webrpg
COPY --from=builder /app/target/site ./site
COPY --from=builder /app/help ./help
COPY --from=builder /app/modules ./modules

# Persistent state: the SQLite database and uploaded media live under /app/data
# so a single volume survives image upgrades. Migrations run at startup.
RUN mkdir -p /app/data/uploads/media && chown -R webrpg:webrpg /app
VOLUME ["/app/data"]

ENV LEPTOS_OUTPUT_NAME=webrpg \
    LEPTOS_SITE_ROOT=/app/site \
    LEPTOS_SITE_PKG_DIR=pkg \
    LEPTOS_SITE_ADDR=0.0.0.0:3000 \
    DATABASE_URL=/app/data/database.db \
    MEDIA_DIR=/app/data/uploads/media \
    MODULES_DIR=/app/modules

# SECRET_KEY has no default on purpose: it must be supplied at run time and kept
# stable, since it signs the auth cookies.
EXPOSE 3000
USER webrpg
ENTRYPOINT ["/app/webrpg"]
