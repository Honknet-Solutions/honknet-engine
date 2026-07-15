#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

CLIENTS="${HONKNET_SOAK_CLIENTS:-200}"
ENTITIES="${HONKNET_SOAK_ENTITIES:-100000}"
DURATION="${HONKNET_SOAK_DURATION_SECONDS:-21600}"
SAVE_ROOT="$(mktemp -d)"
LOG_FILE="${HONKNET_SOAK_LOG:-$ROOT/soak-server.log}"
SERVER_PID=""

cleanup() {
  if [[ -n "$SERVER_PID" ]]; then
    kill -TERM "$SERVER_PID" 2>/dev/null || true
    wait "$SERVER_PID" 2>/dev/null || true
  fi
  rm -rf "$SAVE_ROOT"
}
trap cleanup EXIT INT TERM

HONKNET_SYNTHETIC_ENTITY_COUNT="$ENTITIES" \
HONKNET_SAVE_ROOT="$SAVE_ROOT" \
HONKNET_AUTOSAVE_SECONDS=900 \
HONKNET_AUTH_REQUIRED=false \
RUST_LOG=info \
./target/release/honknet-server >"$LOG_FILE" 2>&1 &
SERVER_PID=$!

for _ in $(seq 1 120); do
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
curl --fail --silent http://127.0.0.1:3016/metrics > soak-metrics.prom

echo "Soak test passed: clients=$CLIENTS entities=$ENTITIES duration=$DURATION"
echo "Metrics: $ROOT/soak-metrics.prom"
echo "Server log: $LOG_FILE"
