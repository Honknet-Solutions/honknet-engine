import './style.css';

import { ClientConnection } from './connection';
import { getOrCreateGuestIdentityId } from './identity';
import { InputController } from './input';
import {
  PixiRenderer,
  type PixiRendererState,
} from './pixiRenderer';
import type {
  EntityNetId,
  EntitySnapshot,
  ServerMessage,
  Vec2,
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
        <input
          id="server-url"
          value="${DEFAULT_SERVER_URL}"
        />
      </label>

      <button id="connect">Connect</button>

      <div class="status-grid">
        <div>
          <span class="status-label">Identity</span>
          <strong id="identity-status">-</strong>
        </div>

        <div>
          <span class="status-label">Client</span>
          <strong id="client-status">
            not connected
          </strong>
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

      <div id="viewport"></div>

      <p class="hint">
        Use WASD or arrow keys after connecting.
      </p>

      <pre id="log">Client booted. Waiting for connection.</pre>
    </section>
  </main>
`;

const log =
  document.querySelector<HTMLPreElement>('#log');

const connectButton =
  document.querySelector<HTMLButtonElement>(
    '#connect',
  );

const serverUrlInput =
  document.querySelector<HTMLInputElement>(
    '#server-url',
  );

const viewportElement =
  document.querySelector<HTMLElement>('#viewport');

const identityStatus =
  document.querySelector<HTMLElement>(
    '#identity-status',
  );

const clientStatus =
  document.querySelector<HTMLElement>(
    '#client-status',
  );

const entityStatus =
  document.querySelector<HTMLElement>(
    '#entity-status',
  );

const tickStatus =
  document.querySelector<HTMLElement>(
    '#tick-status',
  );

if (!viewportElement) {
  throw new Error('Missing #viewport element');
}

let clientId: string | null = null;
let playerEntityNetId: EntityNetId | null = null;
let lastServerTick: number | null = null;
let inputSeq = 0;

let lastSentMovement: Vec2 = {
  x: 0,
  y: 0,
};

const entitiesByNetId = new Map<
  EntityNetId,
  EntitySnapshot
>();

const inputController = new InputController();

const pixiRenderer = new PixiRenderer(
  viewportElement,
);

const playerIdentityId =
  getOrCreateGuestIdentityId();

const connection = new ClientConnection({
  onOpen: () => {
    writeLog(
      'WebSocket opened. Sending Hello.',
    );

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

    lastSentMovement = {
      x: 0,
      y: 0,
    };

    setText(clientStatus, 'disconnected');
    setText(entityStatus, '-');

    updateRendererState();
  },

  onError: (message) => {
    writeLog(message);
  },
});

await pixiRenderer.initialize();

setText(identityStatus, playerIdentityId);
writeLog(`Guest identity: ${playerIdentityId}`);

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

  const currentLines =
    log.textContent?.split('\n') ?? [];

  const nextLines = [
    ...currentLines,
    `${new Date().toLocaleTimeString()} ${message}`,
  ];

  log.textContent = nextLines
    .slice(-18)
    .join('\n');
}

function sendCurrentInput(): void {
  const movement =
    inputController.getMovement();

  updateRendererState();

  if (
    !connection.isConnected ||
    playerEntityNetId === null
  ) {
    return;
  }

  if (
    movement.x === lastSentMovement.x &&
    movement.y === lastSentMovement.y
  ) {
    return;
  }

  inputSeq += 1;

  const sent = connection.send({
    type: 'Input',
    data: {
      seq: inputSeq,
      movement,
    },
  });

  if (!sent) {
    return;
  }

  lastSentMovement = {
    x: movement.x,
    y: movement.y,
  };
}

function handleServerMessage(
  message: ServerMessage,
): void {
  switch (message.type) {
    case 'Welcome':
      clientId = message.data.client_id;
      playerEntityNetId =
        message.data.entity_net_id;

      lastSentMovement = {
        x: Number.NaN,
        y: Number.NaN,
      };

      setText(clientStatus, clientId);

      setText(
        entityStatus,
        String(playerEntityNetId),
      );

      writeLog(
        `Welcome received. client_id=${clientId}, player_entity=${playerEntityNetId}`,
      );

      updateRendererState();
      break;

    case 'Snapshot':
      lastServerTick = message.data.tick;

      setText(
        tickStatus,
        String(lastServerTick),
      );

      const acknowledgedInputSeq =
        message.data.last_processed_input_seq;

      entitiesByNetId.clear();

      for (const entity of message.data.entities) {
        entitiesByNetId.set(
          entity.net_id,
          entity,
        );
      }

      writeLog(
        `Snapshot tick=${message.data.tick}, ack=${acknowledgedInputSeq ?? 'none'}, entities=${message.data.entities.length}`,
      );

      updateRendererState();
      break;

    case 'Chat':
      writeLog(
        `[${message.data.from}] ${message.data.text}`,
      );
      break;

    case 'Error':
      writeLog(
        `Server error: ${message.data.message}`,
      );
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
  const rendererState: PixiRendererState = {
    serverTick: lastServerTick,
    playerEntityNetId,
    movement: inputController.getMovement(),
    entities: entitiesByNetId,
  };

  pixiRenderer.update(rendererState);
}

function connectToServer(): void {
  if (
    connection.isConnected ||
    connection.isConnecting
  ) {
    writeLog(
      'Connection is already active.',
    );

    return;
  }

  const serverUrl =
    serverUrlInput?.value.trim() ||
    DEFAULT_SERVER_URL;

  writeLog(`Connecting to ${serverUrl} ...`);

  const started =
    connection.connect(serverUrl);

  if (!started) {
    writeLog(
      'Failed to start connection.',
    );
  }
}

connectButton?.addEventListener(
  'click',
  () => {
    connectToServer();
    window.focus();
  },
);

window.addEventListener(
  'beforeunload',
  () => {
    pixiRenderer.destroy();
    inputController.destroy();
    connection.disconnect();
  },
);