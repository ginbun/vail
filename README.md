# Vail

Vail is a bastion host project with a Rust backend (`vail-rs`) and an Orion-compatible frontend (`vail-web`).

## Project Layout

- `vail-rs/`: Rust + Axum backend (auth, permissions, host/session APIs, migrations)
- `vail-web/`: Orion-compatible frontend (Vue 3 + Arco direction)
- `orion-visor/`: reference-only upstream project, do not commit changes to this repo

## Local Development

### One-command build

```bash
./build.sh
```

This builds `vail-web`, syncs frontend static assets into `vail-rs/web-dist`, then compiles backend release binary.

### Backend

```bash
cd vail-rs
cargo check
```

### JWT Ed25519 keypair generation (EdDSA)

`vail-rs` supports JWT signing with `EdDSA` (Ed25519) and `HS256` fallback. To generate an Ed25519 keypair for JWT:

JWT configuration is either-or:

- `HS256`: set `VAIL_JWT_ALGORITHM=HS256` and provide `VAIL_JWT_SECRET` only.
- `EdDSA`: set `VAIL_JWT_ALGORITHM=EdDSA` and provide both `VAIL_JWT_PRIVATE_KEY` and `VAIL_JWT_PUBLIC_KEY` as base64 key strings.

```bash
./vail-rs/scripts/generate_jwt_ed25519_keypair.sh
```

By default this writes:

- `vail-rs/.local/jwt/jwt-ed25519-private.b64`
- `vail-rs/.local/jwt/jwt-ed25519-public.b64`

You can also choose a custom output directory:

```bash
./vail-rs/scripts/generate_jwt_ed25519_keypair.sh /path/to/output-dir
```

The script writes base64 key strings to disk. `vail-rs` itself does not read key file paths from config; it expects the environment variables to contain the base64 string content. The generated private key is PKCS8 DER encoded in base64, and the generated public key is the raw Ed25519 public key encoded in base64. Wire the generated keys into backend env vars like this:

```bash
export VAIL_JWT_ALGORITHM=EdDSA
export VAIL_JWT_PRIVATE_KEY="$(cat vail-rs/.local/jwt/jwt-ed25519-private.b64)"
export VAIL_JWT_PUBLIC_KEY="$(cat vail-rs/.local/jwt/jwt-ed25519-public.b64)"
```

`VAIL_JWT_PRIVATE_KEY` and `VAIL_JWT_PUBLIC_KEY` must contain base64 key strings. PEM is not supported.

> Note: Ed25519 in this section is used for JWT signature (EdDSA), not data encryption.

### Data Encryption Key

`VAIL_DATA_ENCRYPTION_KEY` is separate from JWT keys. It is used by `vail-rs` to encrypt sensitive data before storing it in the database, such as host credentials and SSH private-key material.

Generate a strong random key with OpenSSL:

```bash
openssl rand -hex 32
```

You can also use base64 output if preferred:

```bash
openssl rand -base64 48
```

Example export for local or container deployment:

```bash
export VAIL_DATA_ENCRYPTION_KEY="$(openssl rand -hex 32)"
```

Important notes:

- Use a high-entropy value with at least 32 characters.
- Store it securely in your secret manager, `.env`, or deployment secret store.
- Keep it stable per environment. If you change it later, previously encrypted data may no longer be decryptable.

### Frontend

```bash
cd vail-web
npm ci
npm run build
```

Note: frontend direction is compatibility-first with Orion UI behavior and API contract, then incremental UX/performance optimization.

## Docker Compose

The repository includes `docker-compose.yaml` for PostgreSQL 18 + backend.
Frontend static files are built from `vail-web` and embedded into the backend binary with `rust-embed`.

Container base images:

- Backend build: `rust:1.94-trixie`
- Frontend build stage (inside backend build): `node:24-alpine`
- Backend runtime: `gcr.io/distroless/base-debian13`

```bash
docker compose up --build
```

After startup:

- Web UI + API entrypoint: `http://localhost:3000`
- PostgreSQL: `localhost:5432` (`vail` / `vail`)

The backend runs migrations automatically on startup and ensures weekly partitions for log/session tables.
API endpoints are available under `/api/v1/*`.

## CI (GitHub Actions)

CI workflow path: `.github/workflows/ci.yml`

- Backend job: `cargo check` + `cargo test --lib`
- Frontend job: `npm ci` + `npm run build`

## Versioning and Release

- Every GitHub Release is treated as one product version.
- Release tags must follow semantic version format: `vMAJOR.MINOR.PATCH`.
- Release workflow path: `.github/workflows/release.yml`
- On release publish, workflow validates tag format and runs backend/frontend verification.

## Current Focus

First visible feature set:

- My Hosts visibility
- My Permissions visibility
- Bastion-side authorization enforcement

## TODO

- [ ] JIT (Just-In-Time) privilege elevation workflow (request/approve/auto-expire)
- [ ] Risk-based command control for high-risk production actions
- [ ] Session replay and richer audit visualization
