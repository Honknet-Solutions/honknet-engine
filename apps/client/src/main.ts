import './style.css';

import { ClientConnection } from './connection';
import {
  DebugRenderer,
  type DebugRendererState,
} from './debugRenderer';
import { getOrCreateGuestIdentityId } from './identity';
import { InputController } from './input';
import type {
  EntityNetId,
  EntitySnapshot,
  ServerMessage,
} from './protocol';

const CLIENT_VERSION = '0.1.0-dev';
const DEFAULT_SERVER_URL = 'ws://127.0.0.1:3015';
const INPUT_SEND_INTERVAL_MS = 50;

const app = document.querySelector<HTMLDivElement>('#app');

if (!app) {
  throw new Error('Missing #app root element');
}

app.innerHTML = `
  <main class="shell">
    <section class="panel">
      <p class="eyebrow">Honknet Solutions</p>
      <h1>Space Station 15</h1>

      <p>
        Browser-first modular 2D multiplayer immersive simulation framework.
      </p>

      <label class="field">
        <span>Server URL</span>
        <input id="server-url" value="${DEFAULT_SERVER_URL}" />
      </label>

      <button id="connect">Connect</button>

      <div class="status-grid">
        <div>
          <span class="status-label">Identity</span>
          <strong id="identity-status">-</strong>
        </div>

        <div>
          <span class="status-label">Client</span>
          <strong id="client-status">not connected</strong>
        </div>

        <div>
          <span class="status-label">Entity</span>
          <strong id="entity-status">-</strong>
        </div>

        <div>
          <span class="status-label">Tick</span>
          <strong id="tick-status">-</strong>
        </div>
      </div>

      <canvas id="viewport" width="800" height="480"></canvas>

      <p class="hint">
        Use WASD or arrow keys after connecting.
      </p>

      <pre id="log">Client booted. Waiting for connection.</pre>
    </section>
  </main>
`;

const log = document.querySelector<HTMLPreElement>('#log');

const connectButton =
  document.querySelector<HTMLButtonElement>('#connect');

const serverUrlInput =
  document.querySelector<HTMLInputElement>('#server-url');

const canvasElement =
  document.querySelector<HTMLCanvasElement>('#viewport');

const identityStatus =
  document.querySelector<HTMLElement>('#identity-status');

const clientStatus =
  document.querySelector<HTMLElement>('#client-status');

const entityStatus =
  document.querySelector<HTMLElement>('#entity-status');

const tickStatus =
  document.querySelector<HTMLElement>('#tick-status');

if (!canvasElement) {
  throw new Error('Missing #viewport canvas');
}

let clientId: string | null = null;
let playerEntityNetId: EntityNetId | null = null;
let lastServerTick: number | null = null;
let inputSeq = 0;

const entitiesByNetId = new Map<
  EntityNetId,
  EntitySnapshot
>();

const inputController = new InputController();
const debugRenderer = new DebugRenderer(canvasElement);
const playerIdentityId = getOrCreateGuestIdentityId();

const connection = new ClientConnection({
  onOpen: () => {
    writeLog('WebSocket opened. Sending Hello.');

    connection.send({
      type: 'Hello',
      data: {
        client_version: CLIENT_VERSION,
        identity_id: playerIdentityId,
      },
    });
  },

  onMessage: (message) => {
    handleServerMessage(message);
  },

  onClose: () => {
    writeLog('WebSocket closed.');

    clientId = null;
    playerEntityNetId = null;

    setText(clientStatus, 'disconnected');
    setText(entityStatus, '-');

    updateRendererState();
  },

  onError: (message) => {
    writeLog(message);
  },
});

setText(identityStatus, playerIdentityId);
writeLog(`Guest identity: ${playerIdentityId}`);

debugRenderer.start();
updateRendererState();

window.setInterval(() => {
  sendCurrentInput();
}, INPUT_SEND_INTERVAL_MS);

function setText(
  element: HTMLElement | null,
  value: string,
): void {
  if (!element) {
    return;
  }

  element.textContent = value;
}

function writeLog(message: string): void {
  if (!log) {
    return;
  }

  const currentLines = log.textContent?.split('\n') ?? [];

  const nextLines = [
    ...currentLines,
    `${new Date().toLocaleTimeString()} ${message}`,
  ];

  log.textContent = nextLines.slice(-18).join('\n');
}

function sendCurrentInput(): void {
  if (
    !connection.isConnected ||
    playerEntityNetId === null
  ) {
    updateRendererState();
    return;
  }

  const movement = inputController.getMovement();

  updateRendererState();

  if (movement.x === 0 && movement.y === 0) {
    return;
  }

  inputSeq += 1;

  connection.send({
    type: 'Input',
    data: {
      seq: inputSeq,
      movement,
    },
  });
}

function handleServerMessage(message: ServerMessage): void {
  switch (message.type) {
    case 'Welcome':
      clientId = message.data.client_id;
      playerEntityNetId = message.data.entity_net_id;

      setText(clientStatus, clientId);
      setText(entityStatus, String(playerEntityNetId));

      writeLog(
        `Welcome received. client_id=${clientId}, player_entity=${playerEntityNetId}`,
      );

      updateRendererState();
      break;

    case 'Snapshot':
      lastServerTick = message.data.tick;

      setText(tickStatus, String(lastServerTick));

      entitiesByNetId.clear();

      for (const entity of message.data.entities) {
        entitiesByNetId.set(entity.net_id, entity);
      }

      writeLog(
        `Snapshot tick=${message.data.tick}, entities=${message.data.entities.length}`,
      );

      updateRendererState();
      break;

    case 'Chat':
      writeLog(`[${message.data.from}] ${message.data.text}`);
      break;

    case 'Error':
      writeLog(`Server error: ${message.data.message}`);
      break;

    default: {
      const unreachable: never = message;

      writeLog(
        `Unknown server message: ${JSON.stringify(unreachable)}`,
      );
    }
  }
}

function updateRendererState(): void {
  const rendererState: DebugRendererState = {
    serverTick: lastServerTick,
    playerEntityNetId,
    movement: inputController.getMovement(),
    entities: entitiesByNetId,
  };

  debugRenderer.update(rendererState);
}

function connectToServer(): void {
  if (connection.isConnected || connection.isConnecting) {
    writeLog('Connection is already active.');
    return;
  }

  const serverUrl =
    serverUrlInput?.value.trim() || DEFAULT_SERVER_URL;

  writeLog(`Connecting to ${serverUrl} ...`);

  const started = connection.connect(serverUrl);

  if (!started) {
    writeLog('Failed to start connection.');
  }
}

connectButton?.addEventListener('click', () => {
  connectToServer();
  window.focus();
});

window.addEventListener('beforeunload', () => {
  debugRenderer.stop();
  inputController.destroy();
  connection.disconnect();
});