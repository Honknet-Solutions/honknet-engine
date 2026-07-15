#!/usr/bin/env node
import { createHmac, randomUUID } from 'node:crypto';
import { performance } from 'node:perf_hooks';

const url = process.argv[2] ?? 'ws://127.0.0.1:3015';
const clientCount = positiveInteger(process.argv[3], 30);
const durationSeconds = positiveInteger(process.argv[4], 60);
const protocolVersion = 4;
const clientVersion = 'load-test-1';
const authSecret = process.env.HONKNET_AUTH_SECRET ?? null;
const authRequired = parseBoolean(process.env.HONKNET_AUTH_REQUIRED, false);

if (authRequired && (!authSecret || Buffer.byteLength(authSecret) < 32)) {
  fail('HONKNET_AUTH_SECRET must contain at least 32 bytes when auth is required.');
}

const stats = {
  connected: 0,
  welcomed: 0,
  disconnected: 0,
  messages: 0,
  snapshots: 0,
  deltas: 0,
  errors: [],
  latencies: [],
};

const clients = Array.from({ length: clientCount }, (_, index) => createClient(index));
await Promise.all(clients.map((client) => client.ready));

if (stats.welcomed !== clientCount) {
  fail(`Only ${stats.welcomed}/${clientCount} clients completed the handshake.`);
}

const started = performance.now();
await sleep(durationSeconds * 1000);
for (const client of clients) client.close();
await Promise.allSettled(clients.map((client) => client.closed));
const elapsed = (performance.now() - started) / 1000;

const sortedLatencies = stats.latencies.toSorted((left, right) => left - right);
const percentile = (value) =>
  sortedLatencies.length === 0
    ? 0
    : sortedLatencies[Math.min(sortedLatencies.length - 1, Math.floor(sortedLatencies.length * value))];

const report = {
  url,
  clients: clientCount,
  requestedDurationSeconds: durationSeconds,
  elapsedSeconds: Number(elapsed.toFixed(2)),
  connected: stats.connected,
  welcomed: stats.welcomed,
  disconnected: stats.disconnected,
  messages: stats.messages,
  snapshots: stats.snapshots,
  deltas: stats.deltas,
  pingLatencyMs: {
    p50: Number(percentile(0.5).toFixed(2)),
    p95: Number(percentile(0.95).toFixed(2)),
    p99: Number(percentile(0.99).toFixed(2)),
  },
  errors: stats.errors,
};
console.log(JSON.stringify(report, null, 2));

if (stats.errors.length > 0 || stats.disconnected !== clientCount) {
  process.exitCode = 1;
}

function createClient(index) {
  const identity = `load-${index}-${randomUUID()}`;
  const token = authSecret ? issueToken(identity, 3_600) : null;
  const socket = new WebSocket(url);
  let entityId = null;
  let inputSequence = 0;
  let clientTick = 0;
  let baselineTick = null;
  let movementTimer = null;
  let pingTimer = null;
  let directionPhase = index % 4;
  const pendingPings = new Map();

  let resolveReady;
  let rejectReady;
  const ready = new Promise((resolve, reject) => {
    resolveReady = resolve;
    rejectReady = reject;
  });
  let resolveClosed;
  const closed = new Promise((resolve) => {
    resolveClosed = resolve;
  });

  const handshakeTimer = setTimeout(() => {
    rejectReady(new Error(`Client ${index} handshake timed out`));
    socket.close();
  }, 10_000);

  socket.addEventListener('open', () => {
    stats.connected += 1;
    socket.send(JSON.stringify({
      type: 'Hello',
      data: {
        protocol_version: protocolVersion,
        client_version: clientVersion,
        identity_id: identity,
        auth_token: token,
      },
    }));
  });

  socket.addEventListener('message', (event) => {
    stats.messages += 1;
    const message = JSON.parse(String(event.data));
    switch (message.type) {
      case 'Welcome':
        entityId = message.data.entity_net_id;
        stats.welcomed += 1;
        clearTimeout(handshakeTimer);
        resolveReady();
        movementTimer = setInterval(sendMovement, 1000 / 30);
        pingTimer = setInterval(sendPing, 1000);
        break;
      case 'Snapshot':
        stats.snapshots += 1;
        baselineTick = message.data.tick;
        acknowledge(message.data.tick);
        break;
      case 'StateDelta':
        stats.deltas += 1;
        if (baselineTick !== message.data.baseline_tick) {
          socket.send(JSON.stringify({ type: 'RequestFullState' }));
          baselineTick = null;
        } else {
          baselineTick = message.data.tick;
          acknowledge(message.data.tick);
        }
        break;
      case 'Pong': {
        const sentAt = pendingPings.get(message.data.nonce);
        if (sentAt !== undefined) {
          pendingPings.delete(message.data.nonce);
          stats.latencies.push(performance.now() - sentAt);
        }
        break;
      }
      case 'Error':
        stats.errors.push({ client: index, ...message.data });
        if (message.data.fatal) socket.close();
        break;
      default:
        break;
    }
  });

  socket.addEventListener('error', () => {
    const error = new Error(`Client ${index} WebSocket error`);
    stats.errors.push({ client: index, code: 'websocket.error', message: error.message });
    rejectReady(error);
  });

  socket.addEventListener('close', () => {
    clearTimeout(handshakeTimer);
    clearInterval(movementTimer);
    clearInterval(pingTimer);
    stats.disconnected += 1;
    resolveClosed();
  });

  function sendMovement() {
    if (socket.readyState !== WebSocket.OPEN || entityId === null) return;
    inputSequence = (inputSequence + 1) >>> 0;
    clientTick = (clientTick + 1) >>> 0;
    if (clientTick % 90 === 0) directionPhase = (directionPhase + 1) % 4;
    const movement = [
      { x: 1, y: 0 },
      { x: 0, y: 1 },
      { x: -1, y: 0 },
      { x: 0, y: -1 },
    ][directionPhase];
    socket.send(JSON.stringify({
      type: 'Input',
      data: { seq: inputSequence, client_tick: clientTick, movement },
    }));
  }

  function acknowledge(tick) {
    socket.send(JSON.stringify({ type: 'SnapshotAck', data: { tick } }));
  }

  function sendPing() {
    if (socket.readyState !== WebSocket.OPEN) return;
    const nonce = Math.floor(Math.random() * Number.MAX_SAFE_INTEGER);
    pendingPings.set(nonce, performance.now());
    socket.send(JSON.stringify({ type: 'Ping', data: { nonce } }));
  }

  return {
    ready,
    closed,
    close() {
      if (socket.readyState === WebSocket.OPEN || socket.readyState === WebSocket.CONNECTING) {
        socket.close(1000, 'load test complete');
      }
    },
  };
}

function issueToken(identity, lifetimeSeconds) {
  const expires = Math.floor(Date.now() / 1000) + lifetimeSeconds;
  const payload = `v1\n${identity}\n${expires}`;
  const signature = createHmac('sha256', authSecret).update(payload).digest('hex');
  return `v1:${expires}:${signature}`;
}

function positiveInteger(raw, fallback) {
  const value = Number.parseInt(raw ?? '', 10);
  return Number.isSafeInteger(value) && value > 0 ? value : fallback;
}

function parseBoolean(raw, fallback) {
  if (raw == null) return fallback;
  return ['1', 'true', 'yes', 'on'].includes(raw.trim().toLowerCase());
}

function sleep(milliseconds) {
  return new Promise((resolve) => setTimeout(resolve, milliseconds));
}

function fail(message) {
  console.error(message);
  process.exit(1);
}
