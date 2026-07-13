import './style.css';

import { ClientConnection } from './connection';
import { ClientWorld } from './clientWorld';
import { getOrCreateGuestIdentityId } from './identity';
import { InputController } from './input';
import {
  LocalPlayerController,
  type LocalPlayerControllerState,
} from './localPlayerController';
import {
  PixiRenderer,
  type PixiRendererState,
} from './pixiRenderer';
import type {
  EntityNetId,
  ServerMessage,
} from './protocol';

const CLIENT_VERSION = '0.1.0-dev';

const DEFAULT_SERVER_URL =
  'ws://127.0.0.1:3015';

const app =
  document.querySelector<HTMLDivElement>(
    '#app',
  );

if (!app) {
  throw new Error(
    'Missing #app root element',
  );
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
  document.querySelector<HTMLPreElement>(
    '#log',
  );

const connectButton =
  document.querySelector<HTMLButtonElement>(
    '#connect',
  );

const serverUrlInput =
  document.querySelector<HTMLInputElement>(
    '#server-url',
  );

const viewportElement =
  document.querySelector<HTMLElement>(
    '#viewport',
  );

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
  throw new Error(
    'Missing #viewport element',
  );
}

let clientId: string | null = null;

let playerEntityNetId:
  EntityNetId | null = null;

let localPlayerState:
  LocalPlayerControllerState = {
    clientSimulationTick: 0,
    lastProcessedInputSeq: null,
    lastProcessedClientTick: null,
    predictedPlayerPosition: null,
    pendingInputCount: 0,
  };

const clientWorld =
  new ClientWorld();

const inputController =
  new InputController();

const pixiRenderer =
  new PixiRenderer(
    viewportElement,
  );

const playerIdentityId =
  getOrCreateGuestIdentityId();

const connection =
  new ClientConnection({
    onOpen: () => {
      writeLog(
        'WebSocket opened. Sending Hello.',
      );

      connection.send({
        type: 'Hello',
        data: {
          client_version:
            CLIENT_VERSION,

          identity_id:
            playerIdentityId,
        },
      });
    },

    onMessage: (message) => {
      handleServerMessage(message);
    },

    onClose: () => {
      writeLog(
        'WebSocket closed.',
      );

      clientId = null;

      playerEntityNetId = null;

      clientWorld.clear();

      localPlayerController
        .clearPlayer();

      setText(
        clientStatus,
        'disconnected',
      );

      setText(
        entityStatus,
        '-',
      );

      setText(
        tickStatus,
        '-',
      );

      updateRendererState();
    },

    onError: (message) => {
      writeLog(message);
    },
  });

const localPlayerController =
  new LocalPlayerController({
    getMovement: () =>
      inputController.getMovement(),

    isConnected: () =>
      connection.isConnected,

    sendMessage: (message) =>
      connection.send(message),

    onFrame: (state) => {
      localPlayerState = state;

      updateRendererState();
    },

    onPredictionSnap: (
      errorDistance,
    ) => {
      writeLog(
        `Prediction snapped to server. error=${errorDistance.toFixed(3)}`,
      );
    },
  });

await pixiRenderer.initialize();

localPlayerController.start();

setText(
  identityStatus,
  playerIdentityId,
);

writeLog(
  `Guest identity: ${playerIdentityId}`,
);

updateRendererState();

function setText(
  element: HTMLElement | null,
  value: string,
): void {
  if (!element) {
    return;
  }

  element.textContent = value;
}

function writeLog(
  message: string,
): void {
  if (!log) {
    return;
  }

  const currentLines =
    log.textContent?.split('\n') ??
    [];

  const nextLines = [
    ...currentLines,
    `${new Date().toLocaleTimeString()} ${message}`,
  ];

  log.textContent =
    nextLines
      .slice(-18)
      .join('\n');
}

function handleServerMessage(
  message: ServerMessage,
): void {
  switch (message.type) {
    case 'Welcome':
      handleWelcome(message);
      break;

    case 'Snapshot':
      handleSnapshot(message);
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
      const unreachable: never =
        message;

      writeLog(
        `Unknown server message: ${JSON.stringify(unreachable)}`,
      );
    }
  }
}

function handleWelcome(
  message: Extract<
    ServerMessage,
    { type: 'Welcome' }
  >,
): void {
  clientId =
    message.data.client_id;

  playerEntityNetId =
    message.data.entity_net_id;

  clientWorld.clear();

  localPlayerController
    .setPlayerEntity(
      playerEntityNetId,
    );

  setText(
    clientStatus,
    clientId,
  );

  setText(
    entityStatus,
    String(playerEntityNetId),
  );

  setText(
    tickStatus,
    '-',
  );

  writeLog(
    `Welcome received. client_id=${clientId}, player_entity=${playerEntityNetId}`,
  );

  updateRendererState();
}

function handleSnapshot(
  message: Extract<
    ServerMessage,
    { type: 'Snapshot' }
  >,
): void {
  const snapshotResult =
    clientWorld.applySnapshot(
      message.data.tick,
      message.data.entities,
    );

  const serverTick =
    clientWorld.getServerTick();

  setText(
    tickStatus,
    serverTick === null
      ? '-'
      : String(serverTick),
  );

  const playerEntity =
    playerEntityNetId === null
      ? undefined
      : clientWorld.getEntity(
          playerEntityNetId,
        );

  localPlayerController
    .handleSnapshot(
      playerEntity,
      message.data
        .last_processed_input_seq,
      message.data
        .last_processed_client_tick,
    );

  const currentLocalState =
    localPlayerController.getState();

  writeLog(
    `Snapshot serverTick=${serverTick ?? 'none'}, clientTick=${currentLocalState.clientSimulationTick}, ackSeq=${currentLocalState.lastProcessedInputSeq ?? 'none'}, ackClientTick=${currentLocalState.lastProcessedClientTick ?? 'none'}, pending=${currentLocalState.pendingInputCount}, entities=${clientWorld.getEntityCount()}, created=${snapshotResult.createdEntityIds.length}, updated=${snapshotResult.updatedEntityIds.length}, removed=${snapshotResult.removedEntityIds.length}`,
  );

  updateRendererState();
}

function updateRendererState(): void {
  const worldState =
    clientWorld.getState();

  const rendererState:
    PixiRendererState = {
      serverTick:
        worldState.serverTick,

      playerEntityNetId,

      movement:
        inputController.getMovement(),

      predictedPlayerPosition:
        localPlayerState
          .predictedPlayerPosition,

      entities:
        worldState.entities,
    };

  pixiRenderer.update(
    rendererState,
  );
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

  writeLog(
    `Connecting to ${serverUrl} ...`,
  );

  const started =
    connection.connect(
      serverUrl,
    );

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
    localPlayerController.stop();

    pixiRenderer.destroy();

    inputController.destroy();

    connection.disconnect();

    clientWorld.clear();
  },
);