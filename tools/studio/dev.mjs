import { createServer } from 'vite';
import { WebSocketServer } from 'ws';

const relayPort = Number(process.env.HONKNET_HOT_RELOAD_PORT ?? 3016);
const relay = new WebSocketServer({ port: relayPort });
relay.on('connection', (socket) => {
  socket.on('message', (payload, isBinary) => {
    if (isBinary) return;
    let parsed;
    try { parsed = JSON.parse(payload.toString()); }
    catch { return; }
    if (!parsed || parsed.type !== 'HotReload' || typeof parsed.path !== 'string' || typeof parsed.content !== 'string') return;
    const encoded = JSON.stringify(parsed);
    for (const client of relay.clients) {
      if (client.readyState === client.OPEN) client.send(encoded);
    }
  });
});
relay.on('listening', () => console.log(`Honknet hot reload relay: ws://127.0.0.1:${relayPort}`));
relay.on('error', (error) => console.error('Hot reload relay error:', error));

const vite = await createServer({
  root: new URL('.', import.meta.url).pathname,
  server: { host: '0.0.0.0' },
});
await vite.listen();
vite.printUrls();

async function shutdown() {
  await vite.close();
  await new Promise((resolve) => relay.close(resolve));
  process.exit(0);
}
process.once('SIGINT', () => void shutdown());
process.once('SIGTERM', () => void shutdown());
