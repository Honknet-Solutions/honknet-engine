#!/usr/bin/env bash
set -euo pipefail
cargo run -p honknet-server -- --listen 127.0.0.1:3015 &
SERVER_PID=$!
trap 'kill $SERVER_PID 2>/dev/null || true' EXIT
sleep 2
cargo run -p honknet-headless-client -- --server 127.0.0.1:3015 --ticks 300
