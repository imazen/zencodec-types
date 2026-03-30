#!/usr/bin/env bash
# Download ICC test profiles from R2 to a local cache directory.
# Requires: AWS CLI, R2_ACCESS_KEY_ID, R2_SECRET_ACCESS_KEY, R2_ACCOUNT_ID env vars.
#
# Usage:
#   ./scripts/fetch-profiles.sh              # downloads to ~/.cache/zencodec-icc/
#   ./scripts/fetch-profiles.sh /tmp/icc     # downloads to /tmp/icc/
#
# After fetching, run verification scripts against the cache:
#   rustc scripts/mega_test.rs -o /tmp/mega_test
#   ICC_PROFILES_DIR=~/.cache/zencodec-icc /tmp/mega_test

set -uo pipefail

CACHE_DIR="${1:-${HOME}/.cache/zencodec-icc}"
ENDPOINT="https://${R2_ACCOUNT_ID:?R2_ACCOUNT_ID not set}.r2.cloudflarestorage.com"
BUCKET="codec-corpus"
PREFIX="icc-profiles/"

mkdir -p "$CACHE_DIR"

echo "Syncing ICC profiles from R2 to $CACHE_DIR ..."

# Work around snap AWS CLI XDG_RUNTIME_DIR issue on WSL/containers.
# The snap-confined aws CLI tries to create /run/user/<uid>/ regardless
# of XDG_RUNTIME_DIR. Create it if missing (needs sudo on WSL).
if [ ! -d "/run/user/$(id -u)" ]; then
    sudo mkdir -p "/run/user/$(id -u)" 2>/dev/null && sudo chown "$(id -u)" "/run/user/$(id -u)" 2>/dev/null || true
fi

export AWS_ACCESS_KEY_ID="${R2_ACCESS_KEY_ID}"
export AWS_SECRET_ACCESS_KEY="${R2_SECRET_ACCESS_KEY}"
if ! aws s3 sync "s3://${BUCKET}/${PREFIX}" "$CACHE_DIR/" \
    --endpoint-url "$ENDPOINT" \
    --no-progress; then
    echo "ERROR: aws s3 sync failed. Check R2 credentials." >&2
    exit 1
fi

COUNT=$(find "$CACHE_DIR" -name "*.icc" -o -name "*.icm" | wc -l)
echo "Done: $COUNT profiles in $CACHE_DIR"
