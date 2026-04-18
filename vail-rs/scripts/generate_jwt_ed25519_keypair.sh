#!/usr/bin/env bash
set -euo pipefail

# Generate an Ed25519 keypair for JWT EdDSA signing/verification.
# Usage:
#   ./vail-rs/scripts/generate_jwt_ed25519_keypair.sh [output_dir]
#
# Default output_dir:
#   vail-rs/.local/jwt

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"
OUTPUT_DIR="${1:-${REPO_ROOT}/vail-rs/.local/jwt}"

PRIVATE_KEY_PATH="${OUTPUT_DIR}/jwt-ed25519-private.pem"
PUBLIC_KEY_PATH="${OUTPUT_DIR}/jwt-ed25519-public.pem"

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

openssl genpkey -algorithm Ed25519 -out "${PRIVATE_KEY_PATH}"
openssl pkey -in "${PRIVATE_KEY_PATH}" -pubout -out "${PUBLIC_KEY_PATH}"

chmod 600 "${PRIVATE_KEY_PATH}"

echo "Generated JWT Ed25519 keypair:"
echo "  Private: ${PRIVATE_KEY_PATH}"
echo "  Public : ${PUBLIC_KEY_PATH}"
echo
echo "Use with vail-rs (env vars):"
echo "  export VAIL_JWT_ALGORITHM=EdDSA"
echo "  export VAIL_JWT_PRIVATE_KEY=\"\$(awk '{printf \"%s\\\\n\", \$0}' \"${PRIVATE_KEY_PATH}\")\""
echo "  export VAIL_JWT_PUBLIC_KEY=\"\$(awk '{printf \"%s\\\\n\", \$0}' \"${PUBLIC_KEY_PATH}\")\""
