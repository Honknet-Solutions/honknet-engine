# Multiplayer acceptance plan

## Automated layers

1. Unit tests cover ECS storage, spatial indexing, authentication, persistence, content inheritance, client delta baselines and Studio/HUI behavior.
2. `tools/verify-runtime.sh` starts the release server and drives multiple real WebSocket clients.
3. `tools/load-test.mjs` continuously sends 30 Hz movement, acknowledges state, requests recovery on baseline mismatch and measures ping percentiles.

## Network impairment test matrix

The public-release test environment should add a proxy such as Toxiproxy or `tc netem` and cover:

| Condition | Acceptance |
|---|---|
| 50 ms RTT, no loss | smooth movement, no full-state loop |
| 150 ms RTT, jitter 30 ms | prediction remains controllable |
| temporary 2 s stall | client recovers through full state |
| reconnect with same identity | same player entity restored |
| duplicate identity session | old session is rejected |
| malformed JSON/binary flood | rate limits/disconnect, server stays responsive |
| 30 clients for 6 hours | no crashes, stable memory and TPS |

WebSocket is reliable and ordered, so packet-loss testing primarily validates TCP stalls and reconnect behavior. A later UDP transport requires acknowledged baseline history, selective reliability and separate loss/reordering tests.
