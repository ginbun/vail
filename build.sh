#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WEB_DIR="$ROOT_DIR/vail-web"
BACKEND_DIR="$ROOT_DIR/vail-rs"
WEB_DIST_DIR="$WEB_DIR/dist"
EMBED_DIST_DIR="$BACKEND_DIR/web-dist"

if [[ ! -f "$WEB_DIR/package.json" ]]; then
  echo "vail-web/package.json not found"
  exit 1
fi

if [[ ! -f "$BACKEND_DIR/Cargo.toml" ]]; then
  echo "vail-rs/Cargo.toml not found"
  exit 1
fi

if [[ ! -d "$WEB_DIR/node_modules" ]]; then
  npm ci --legacy-peer-deps --prefix "$WEB_DIR"
fi

npm run build --prefix "$WEB_DIR"

rm -rf "$EMBED_DIST_DIR"
mkdir -p "$EMBED_DIST_DIR"
cp -R "$WEB_DIST_DIR"/. "$EMBED_DIST_DIR"/

cargo build --release --manifest-path "$BACKEND_DIR/Cargo.toml" --bin vail

echo "Build complete: $BACKEND_DIR/target/release/vail"
