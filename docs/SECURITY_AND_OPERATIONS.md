# Security and operations

## Authentication

Development permits guest identities by default. Public deployments should enable signed tokens:

```toml
[auth]
required = true
```

Set a secret outside source control:

```bash
export HONKNET_AUTH_SECRET="$(openssl rand -hex 32)"
```

Issue a temporary token:

```bash
HONKNET_AUTH_SECRET="$HONKNET_AUTH_SECRET" \
  npm run auth:issue -- player-id 3600
```

The token is bound to the exact identity and expiration time. A second connection with the same identity replaces the older session; the stale session cannot disconnect or control the new one.

## Network boundary

The game server speaks plain WebSocket on port 3015. Terminate TLS at nginx, Caddy or another audited reverse proxy and expose only `wss://` to browsers. The example nginx location is in `deploy/nginx-honknet.conf`.

The observability listener provides:

- `GET /healthz`
- `GET /readyz`
- `GET /metrics`

Keep this listener on localhost or a protected monitoring network. It intentionally has no public authentication layer.

## Abuse controls

The server enforces:

- global and per-IP connection caps;
- handshake and idle timeouts;
- maximum WebSocket message/frame sizes;
- malformed-protocol disconnect thresholds;
- token-bucket limits for movement, chat, interaction and UI actions;
- chat, identity, action, session and script-state length limits;
- script command quotas and tick deadlines;
- authoritative movement and server-owned identity/network components.

Reverse-proxy and firewall rate limits remain recommended because application limits do not protect the TCP accept queue or TLS terminator.

## Persistence

Saves are written to a temporary file, synchronized, atomically replaced and backed up as `.bak`. A corrupt primary falls back to the prior valid backup. Backups are not a substitute for off-host snapshots.

Recommended production layout:

```text
/opt/honknet               read-only binaries/content
/etc/honknet               configuration and secret environment file
/var/lib/honknet           writable saves
/var/log/honknet           logs, when not using journald
```

## Monitoring alerts

Useful initial alerts:

- `honknet_tick_overruns_total` increasing continuously;
- `honknet_max_tick_seconds` above the tick budget;
- `honknet_script_failures_total > 0`;
- `honknet_persistence_failures_total > 0`;
- unexpected increases in rejected connections, malformed messages or rate limiting;
- readiness endpoint unavailable.

## Incident recovery

1. Stop accepting new traffic at the reverse proxy.
2. Preserve logs, the primary save and `.bak` save.
3. Restart only after copying the save directory.
4. Verify `/readyz`, then run a small load test.
5. Re-enable traffic gradually and watch tick, script and persistence metrics.
