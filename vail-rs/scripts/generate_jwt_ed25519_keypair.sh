#!/usr/bin/env bash
set -euo pipefail

# Generate an Ed25519 keypair for JWT EdDSA signing/verification.

usage() {
  cat <<'EOF'
Generate an Ed25519 keypair for vail-rs JWT EdDSA signing.

Usage:
  ./vail-rs/scripts/generate_jwt_ed25519_keypair.sh [output_dir]
  ./vail-rs/scripts/generate_jwt_ed25519_keypair.sh --help

Arguments:
  output_dir   Directory to write base64-encoded DER key files into.
               Default: vail-rs/.local/jwt

Output files:
  jwt-ed25519-private.b64
  jwt-ed25519-public.b64

Note:
  The script writes base64 key strings to disk. vail-rs expects the environment
  variables VAIL_JWT_PRIVATE_KEY and VAIL_JWT_PUBLIC_KEY to contain those
  base64 strings, not file paths.
EOF
}

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"

case "${1:-}" in
  -h|--help)
    usage
    exit 0
    ;;
esac

if [[ "$#" -gt 1 ]]; then
  usage >&2
  exit 1
fi

OUTPUT_DIR="${1:-${REPO_ROOT}/vail-rs/.local/jwt}"

PRIVATE_KEY_PATH="${OUTPUT_DIR}/jwt-ed25519-private.b64"
PUBLIC_KEY_PATH="${OUTPUT_DIR}/jwt-ed25519-public.b64"

if ! command -v openssl >/dev/null 2>&1; then
  echo "openssl is required but was not found in PATH" >&2
  exit 1
fi

if [[ -e "${PRIVATE_KEY_PATH}" || -e "${PUBLIC_KEY_PATH}" ]]; then
  echo "Refusing to overwrite existing key files:" >&2
  [[ -e "${PRIVATE_KEY_PATH}" ]] && echo "  - ${PRIVATE_KEY_PATH}" >&2
  [[ -e "${PUBLIC_KEY_PATH}" ]] && echo "  - ${PUBLIC_KEY_PATH}" >&2
  echo "Delete them first or choose another output directory." >&2
  exit 1
fi

mkdir -p "${OUTPUT_DIR}"
umask 077

tmp_private_pem="$(mktemp)"
tmp_public_pem="$(mktemp)"
trap 'rm -f "${tmp_private_pem}" "${tmp_public_pem}"' EXIT

openssl genpkey -algorithm Ed25519 -out "${tmp_private_pem}"
openssl pkey -in "${tmp_private_pem}" -pubout -out "${tmp_public_pem}"

openssl pkey -in "${tmp_private_pem}" -outform DER | base64 -w0 > "${PRIVATE_KEY_PATH}"
openssl pkey -pubin -in "${tmp_public_pem}" -outform DER | tail -c 32 | base64 -w0 > "${PUBLIC_KEY_PATH}"

chmod 600 "${PRIVATE_KEY_PATH}"
chmod 600 "${PUBLIC_KEY_PATH}"

echo "Generated JWT Ed25519 keypair (base64 strings):"
echo "  Private: ${PRIVATE_KEY_PATH}"
echo "  Public : ${PUBLIC_KEY_PATH}"
echo
echo "vail-rs expects base64 strings in env vars, not file paths. Use:"
echo "  export VAIL_JWT_ALGORITHM=EdDSA"
echo "  export VAIL_JWT_PRIVATE_KEY=\"\$(cat \"${PRIVATE_KEY_PATH}\")\""
echo "  export VAIL_JWT_PUBLIC_KEY=\"\$(cat \"${PUBLIC_KEY_PATH}\")\""
