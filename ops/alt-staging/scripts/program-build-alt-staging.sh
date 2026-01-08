#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"
ENV_FILE="$REPO_ROOT/ops/alt-staging/.env"

if [[ -f "$ENV_FILE" ]]; then
  # shellcheck disable=SC1090
  source "$ENV_FILE"
fi

FEATURES="${ALT_STAGING_BUILD_FEATURES:-alt-staging}"
ANCHOR_EXTRA_ARGS="${ALT_STAGING_ANCHOR_BUILD_ARGS:-}"

cd "$REPO_ROOT"

echo "==> Building program (alt staging)"
echo "Repo: $REPO_ROOT"
echo "Features: $FEATURES"

if ! command -v anchor >/dev/null 2>&1; then
  echo "ERROR: anchor not found on PATH" >&2
  exit 1
fi

# Anchor passes anything after `--` to cargo.
if [[ -n "$FEATURES" ]]; then
  # If FEATURES contains commas, cargo is fine with it.
  anchor build $ANCHOR_EXTRA_ARGS -- --features "$FEATURES"
else
  anchor build $ANCHOR_EXTRA_ARGS
fi

echo "==> Build complete"
