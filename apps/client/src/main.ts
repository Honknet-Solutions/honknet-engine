import './style.css';

type Vec2 = {
  x: number;
  y: number;
};

type EntitySnapshot = {
  net_id: number;
  prototype: string;
  position: Vec2;
};

type ClientMessage =
  | { type: 'Hello'; data: { client_version: string } }
  | { type: 'Input'; data: { seq: number; movement: Vec2 } }
  | { type: 'Chat'; data: { text: string } }
  | { type: 'Interact'; data: { target: number } };

type ServerMessage =
  | { type: 'Welcome'; data: { client_id: string } }
  | { type: 'Snapshot'; data: { tick: number; entities: EntitySnapshot[] } }
  | { type: 'Chat'; data: { from: string; text: string } }
  | { type: 'Error'; data: { message: string } };

const CLIENT_VERSION = '0.1.0-dev';
const DEFAULT_SERVER_URL = 'ws://127.0.0.1:3015';

const app = document.querySelector<HTMLDivElement>('#app');

if (!app) {
  throw new Error('Missing #app root element');
}

app.innerHTML = `
  <main class="shell">
    <section class="panel">
      <p class="eyebrow">Open Station</p>
      <h1>Space Station 15</h1>
      <p>Browser-first multiplayer 2D immersive simulation framework.</p>

      <label class="field">
        <span>Server URL</span>
        <input id="server-url" value="${DEFAULT_SERVER_URL}" />
      </label>

      <button id="connect">Connect</button>
      <pre id="log">Client booted. Waiting for connection.</pre>
    </section>
  </main>
`;

const log = document.querySelector<HTMLPreElement>('#log');
const button = document.querySelector<HTMLButtonElement>('#connect');
const serverUrlInput = document.querySelector<HTMLInputElement>('#server-url');

let socket: WebSocket | null = null;

function writeLog(message: string): void {
  if (!log) {
    return;
  }

  log.textContent += `\n${new Date().toLocaleTimeString()} ${message}`;
}

function sendClientMessage(message: ClientMessage): void {
  if (!socket || socket.readyState !== WebSocket.OPEN) {
    writeLog('Cannot send message: WebSocket is not open.');
    return;
  }

  socket.send(JSON.stringify(message));
}

function handleServerMessage(message: ServerMessage): void {
  switch (message.type) {
    case 'Welcome':
      writeLog(`Welcome received. client_id=${message.data.client_id}`);
      break;

    case 'Snapshot':
      writeLog(
        `Snapshot tick=${message.data.tick}, entities=${message.data.entities.length}`,
      );

      for (const entity of message.data.entities) {
        writeLog(
          `Entity ${entity.net_id}: ${entity.prototype} at (${entity.position.x}, ${entity.position.y})`,
        );
      }

      break;

    case 'Chat':
      writeLog(`[${message.data.from}] ${message.data.text}`);
      break;

    case 'Error':
      writeLog(`Server error: ${message.data.message}`);
      break;

    default: {
      const unreachable: never = message;
      writeLog(`Unknown server message: ${JSON.stringify(unreachable)}`);
    }
  }
}

function connectToServer(): void {
  if (socket && socket.readyState === WebSocket.OPEN) {
    writeLog('Already connected.');
    return;
  }

  const serverUrl = serverUrlInput?.value.trim() || DEFAULT_SERVER_URL;
  writeLog(`Connecting to ${serverUrl} ...`);

  socket = new WebSocket(serverUrl);

  socket.addEventListener('open', () => {
    writeLog('WebSocket opened. Sending Hello.');

    sendClientMessage({
      type: 'Hello',
      data: {
        client_version: CLIENT_VERSION,
      },
    });
  });

  socket.addEventListener('message', (event: MessageEvent<string>) => {
    try {
      const serverMessage = JSON.parse(event.data) as ServerMessage;
      handleServerMessage(serverMessage);
    } catch (error) {
      writeLog(`Failed to parse server message: ${String(error)}`);
    }
  });

  socket.addEventListener('close', () => {
    writeLog('WebSocket closed.');
  });

  socket.addEventListener('error', () => {
    writeLog('WebSocket error. Is the Rust server running?');
  });
}

button?.addEventListener('click', () => {
  connectToServer();
});