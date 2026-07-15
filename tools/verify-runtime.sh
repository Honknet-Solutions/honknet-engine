#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

CLIENTS="${HONKNET_VERIFY_CLIENTS:-8}"
DURATION="${HONKNET_VERIFY_DURATION_SECONDS:-15}"
LOG_FILE="${TMPDIR:-/tmp}/honknet-verify-server.log"
SAVE_ROOT="$(mktemp -d)"
SERVER_PID=""

cleanup() {
  if [[ -n "$SERVER_PID" ]]; then
    kill -TERM "$SERVER_PID" 2>/dev/null || true
    wait "$SERVER_PID" 2>/dev/null || true
  fi
  rm -rf "$SAVE_ROOT"
}
trap cleanup EXIT INT TERM

HONKNET_SAVE_ROOT="$SAVE_ROOT" \
HONKNET_AUTOSAVE_SECONDS=3600 \
HONKNET_AUTH_REQUIRED=false \
RUST_LOG=info \
./target/release/honknet-server >"$LOG_FILE" 2>&1 &
SERVER_PID=$!

for _ in $(seq 1 60); do
  if node -e "fetch('http://127.0.0.1:3016/readyz').then(r=>process.exit(r.ok?0:1)).catch(()=>process.exit(1))"; then
    break
  fi
  if ! kill -0 "$SERVER_PID" 2>/dev/null; then
    cat "$LOG_FILE"
    exit 1
  fi
  sleep 0.25
done

node tools/load-test.mjs ws://127.0.0.1:3015 "$CLIENTS" "$DURATION"
node -e "fetch('http://127.0.0.1:3016/metrics').then(async r=>{if(!r.ok)process.exit(1); const t=await r.text(); if(!t.includes('honknet_ticks_total'))process.exit(1)}).catch(()=>process.exit(1))"

echo "Runtime smoke/load verification passed. Server log: $LOG_FILE"
