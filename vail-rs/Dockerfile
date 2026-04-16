FROM node:24-alpine AS web-builder

WORKDIR /build/vail-web

RUN apk add --no-cache build-base zlib-dev autoconf automake libtool nasm pkgconf python3

COPY vail-web/package.json vail-web/package-lock.json ./
RUN npm ci --legacy-peer-deps

COPY vail-web .
RUN npm run build

FROM rust:1.94-trixie AS builder

WORKDIR /build/vail-rs

RUN apt-get update \
    && apt-get install -y --no-install-recommends pkg-config libssl-dev libssh2-1-dev \
    && rm -rf /var/lib/apt/lists/*

COPY vail-rs/Cargo.toml vail-rs/Cargo.lock ./
COPY vail-rs/src ./src
COPY vail-rs/migrations ./migrations
COPY vail-rs/config.docker.toml ./config.docker.toml
COPY --from=web-builder /build/vail-web/dist ./web-dist

RUN cargo build --release --bin vail

FROM gcr.io/distroless/base-debian13

WORKDIR /app

COPY --from=builder /build/vail-rs/target/release/vail /app/vail
COPY --from=builder /build/vail-rs/migrations /app/migrations
COPY vail-rs/config.docker.toml /app/config.toml

COPY --from=builder /usr/lib/x86_64-linux-gnu/libssl.so* /usr/lib/x86_64-linux-gnu/
COPY --from=builder /usr/lib/x86_64-linux-gnu/libcrypto.so* /usr/lib/x86_64-linux-gnu/
COPY --from=builder /usr/lib/x86_64-linux-gnu/libssh2.so* /usr/lib/x86_64-linux-gnu/
COPY --from=builder /usr/lib/x86_64-linux-gnu/libz.so* /usr/lib/x86_64-linux-gnu/
COPY --from=builder /usr/lib/x86_64-linux-gnu/libgcc_s.so* /usr/lib/x86_64-linux-gnu/
COPY --from=builder /usr/lib/x86_64-linux-gnu/libstdc++.so* /usr/lib/x86_64-linux-gnu/
COPY --from=builder /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/

ENV VAIL_CONFIG=/app/config.toml
EXPOSE 8080

USER nonroot:nonroot

CMD ["/app/vail"]
