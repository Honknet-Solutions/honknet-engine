#!/usr/bin/env node
import { createHmac } from 'node:crypto';

const [, , identityArgument, lifetimeArgument = '3600'] = process.argv;
const identity = identityArgument?.trim();
const lifetimeSeconds = Number.parseInt(lifetimeArgument, 10);
const secret = process.env.HONKNET_AUTH_SECRET;

if (!identity) {
  fail('Usage: HONKNET_AUTH_SECRET=<secret> npm run auth:issue -- <identity> [lifetime-seconds]');
}
if (!/^[A-Za-z0-9._:@-]{1,128}$/.test(identity)) {
  fail('Identity contains unsupported characters or exceeds 128 characters.');
}
if (!secret || Buffer.byteLength(secret) < 32) {
  fail('HONKNET_AUTH_SECRET must contain at least 32 bytes.');
}
if (!Number.isSafeInteger(lifetimeSeconds) || lifetimeSeconds < 1) {
  fail('Lifetime must be a positive integer number of seconds.');
}

const expires = Math.floor(Date.now() / 1000) + lifetimeSeconds;
const payload = `v1\n${identity}\n${expires}`;
const signature = createHmac('sha256', secret).update(payload).digest('hex');
process.stdout.write(`v1:${expires}:${signature}\n`);

function fail(message) {
  console.error(message);
  process.exit(1);
}
