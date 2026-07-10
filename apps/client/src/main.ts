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
  NetPosition,
  ServerMessage,
  Vec2,
} from './protocol';

const CLIENT_VERSION = '0.1.0-dev';
const DEFAULT_SERVER_URL = 'ws://127.0.0.1:3015';

const CLIENT_SIMULATION_TICK_RATE = 30;
const CLIENT_SIMULATION_DELTA_SECONDS =
  1 / CLIENT_SIMULATION_TICK_RATE;

const INPUT_HEARTBEAT_INTERVAL_TICKS = 15;

const MAX_FRAME_DELTA_SECONDS = 0.25;
const MAX_SIMULATION_STEPS_PER_FRAME = 8;

const PLAYER_MOVE_SPEED = 4.0;

const RECONCILIATION_SPEED = 12.0;
const RECONCILIATION_IGNORE_DISTANCE = 0.0025;
const RECONCILIATION_SNAP_DISTANCE = 2.0;

type PendingInput = {
  sequence: number;
  clientTick: number;
  movement: Vec2;
  sentAtMilliseconds: number;
};

const app =
  document.querySelector<HTMLDivElement>('#app');

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
  throw new Error('Missing #viewport element');
}

let clientId: string | null = null;
let playerEntityNetId: EntityNetId | null = null;

let lastServerTick: number | null = null;
let lastProcessedInputSeq: number | null = null;
let lastProcessedClientTick: number | null = null;

let inputSeq = 0;
let lastSentMovement: Vec2 | null = null;
let lastInputSendClientTick: number | null = null;

let predictedPlayerPosition:
  NetPosition | null = null;

let pendingPositionCorrection: Vec2 = {
  x: 0,
  y: 0,
};

let clientSimulationTick = 0;
let simulationAccumulatorSeconds = 0;

let lastFrameMilliseconds =
  performance.now();

let simulationFrameRequestId:
  number | null = null;

const pendingInputs: PendingInput[] = [];

const entitiesByNetId = new Map<
  EntityNetId,
  EntitySnapshot
>();

const inputController =
  new InputController();

const pixiRenderer =
  new PixiRenderer(viewportElement);

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

    lastServerTick = null;
    lastProcessedInputSeq = null;
    lastProcessedClientTick = null;

    lastSentMovement = null;
    lastInputSendClientTick = null;
    predictedPlayerPosition = null;

    pendingPositionCorrection = {
      x: 0,
      y: 0,
    };

    pendingInputs.length = 0;
    entitiesByNetId.clear();

    resetClientSimulationClock();

    setText(
      clientStatus,
      'disconnected',
    );

    setText(entityStatus, '-');
    setText(tickStatus, '-');

    updateRendererState();
  },

  onError: (message) => {
    writeLog(message);
  },
});

await pixiRenderer.initialize();

setText(
  identityStatus,
  playerIdentityId,
);

writeLog(
  `Guest identity: ${playerIdentityId}`,
);

updateRendererState();

simulationFrameRequestId =
  requestAnimationFrame(
    updateSimulationFrame,
  );

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
    log.textContent?.split('\n') ?? [];

  const nextLines = [
    ...currentLines,
    `${new Date().toLocaleTimeString()} ${message}`,
  ];

  log.textContent = nextLines
    .slice(-18)
    .join('\n');
}

function updateSimulationFrame(
  currentMilliseconds: number,
): void {
  const frameDeltaSeconds =
    Math.min(
      Math.max(
        (
          currentMilliseconds -
          lastFrameMilliseconds
        ) / 1000,
        0,
      ),
      MAX_FRAME_DELTA_SECONDS,
    );

  lastFrameMilliseconds =
    currentMilliseconds;

  simulationAccumulatorSeconds +=
    frameDeltaSeconds;

  let completedSimulationSteps = 0;

  while (
    simulationAccumulatorSeconds >=
      CLIENT_SIMULATION_DELTA_SECONDS &&
    completedSimulationSteps <
      MAX_SIMULATION_STEPS_PER_FRAME
  ) {
    runClientSimulationTick();

    simulationAccumulatorSeconds -=
      CLIENT_SIMULATION_DELTA_SECONDS;

    completedSimulationSteps += 1;
  }

  if (
    completedSimulationSteps >=
      MAX_SIMULATION_STEPS_PER_FRAME &&
    simulationAccumulatorSeconds >=
      CLIENT_SIMULATION_DELTA_SECONDS
  ) {
    simulationAccumulatorSeconds = 0;
  }

  applyPredictionCorrection(
    frameDeltaSeconds,
  );

  updateRendererState();

  simulationFrameRequestId =
    requestAnimationFrame(
      updateSimulationFrame,
    );
}

function runClientSimulationTick(): void {
  clientSimulationTick =
    (clientSimulationTick + 1) >>> 0;

  const movement =
    normalizeMovement(
      inputController.getMovement(),
    );

  sendMovementInputForTick(
    movement,
    clientSimulationTick,
  );

  applyLocalPrediction(
    movement,
    CLIENT_SIMULATION_DELTA_SECONDS,
  );
}

function sendMovementInputForTick(
  movement: Vec2,
  clientTick: number,
): void {
  if (
    !connection.isConnected ||
    playerEntityNetId === null
  ) {
    return;
  }

  const movementChanged =
    lastSentMovement === null ||
    movement.x !== lastSentMovement.x ||
    movement.y !== lastSentMovement.y;

  const heartbeatRequired =
    lastInputSendClientTick === null ||
    hasReachedTickInterval(
      clientTick,
      lastInputSendClientTick,
      INPUT_HEARTBEAT_INTERVAL_TICKS,
    );

  if (
    !movementChanged &&
    !heartbeatRequired
  ) {
    return;
  }

  inputSeq =
    (inputSeq + 1) >>> 0;

  const sent = connection.send({
    type: 'Input',
    data: {
      seq: inputSeq,
      client_tick: clientTick,
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

  lastInputSendClientTick =
    clientTick;

  pendingInputs.push({
    sequence: inputSeq,
    clientTick,
    movement: {
      x: movement.x,
      y: movement.y,
    },
    sentAtMilliseconds:
      performance.now(),
  });
}

function applyLocalPrediction(
  movement: Vec2,
  deltaSeconds: number,
): void {
  if (
    !connection.isConnected ||
    playerEntityNetId === null ||
    predictedPlayerPosition === null
  ) {
    return;
  }

  predictedPlayerPosition.x +=
    movement.x *
    PLAYER_MOVE_SPEED *
    deltaSeconds;

  predictedPlayerPosition.y +=
    movement.y *
    PLAYER_MOVE_SPEED *
    deltaSeconds;
}

function applyPredictionCorrection(
  deltaSeconds: number,
): void {
  if (
    predictedPlayerPosition === null
  ) {
    return;
  }

  const remainingDistance =
    Math.hypot(
      pendingPositionCorrection.x,
      pendingPositionCorrection.y,
    );

  if (
    remainingDistance <=
    RECONCILIATION_IGNORE_DISTANCE
  ) {
    pendingPositionCorrection = {
      x: 0,
      y: 0,
    };

    return;
  }

  const correctionFactor =
    1 -
    Math.exp(
      -RECONCILIATION_SPEED *
        deltaSeconds,
    );

  const correctionX =
    pendingPositionCorrection.x *
    correctionFactor;

  const correctionY =
    pendingPositionCorrection.y *
    correctionFactor;

  predictedPlayerPosition.x +=
    correctionX;

  predictedPlayerPosition.y +=
    correctionY;

  pendingPositionCorrection.x -=
    correctionX;

  pendingPositionCorrection.y -=
    correctionY;
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

  lastServerTick = null;
  lastProcessedInputSeq = null;
  lastProcessedClientTick = null;

  lastSentMovement = null;
  lastInputSendClientTick = null;
  predictedPlayerPosition = null;

  pendingPositionCorrection = {
    x: 0,
    y: 0,
  };

  pendingInputs.length = 0;

  resetClientSimulationClock();

  setText(
    clientStatus,
    clientId,
  );

  setText(
    entityStatus,
    String(playerEntityNetId),
  );

  setText(tickStatus, '-');

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
  lastServerTick =
    message.data.tick;

  lastProcessedInputSeq =
    message.data
      .last_processed_input_seq;

  lastProcessedClientTick =
    message.data
      .last_processed_client_tick;

  setText(
    tickStatus,
    String(lastServerTick),
  );

  entitiesByNetId.clear();

  for (
    const entity
    of message.data.entities
  ) {
    entitiesByNetId.set(
      entity.net_id,
      entity,
    );
  }

  reconcilePredictedPlayerPosition();

  if (
    lastProcessedInputSeq !== null
  ) {
    removeAcknowledgedInputs(
      lastProcessedInputSeq,
    );
  }

  writeLog(
    `Snapshot serverTick=${message.data.tick}, clientTick=${clientSimulationTick}, ackSeq=${lastProcessedInputSeq ?? 'none'}, ackClientTick=${lastProcessedClientTick ?? 'none'}, pending=${pendingInputs.length}, entities=${message.data.entities.length}`,
  );

  updateRendererState();
}

function reconcilePredictedPlayerPosition(): void {
  if (
    playerEntityNetId === null
  ) {
    return;
  }

  const serverPlayer =
    entitiesByNetId.get(
      playerEntityNetId,
    );

  if (!serverPlayer) {
    return;
  }

  if (
    predictedPlayerPosition === null
  ) {
    predictedPlayerPosition = {
      x: serverPlayer.position.x,
      y: serverPlayer.position.y,
      z: serverPlayer.position.z,
    };

    pendingPositionCorrection = {
      x: 0,
      y: 0,
    };

    return;
  }

  predictedPlayerPosition.z =
    serverPlayer.position.z;

  const errorX =
    serverPlayer.position.x -
    predictedPlayerPosition.x;

  const errorY =
    serverPlayer.position.y -
    predictedPlayerPosition.y;

  const errorDistance =
    Math.hypot(
      errorX,
      errorY,
    );

  if (
    errorDistance <=
    RECONCILIATION_IGNORE_DISTANCE
  ) {
    pendingPositionCorrection = {
      x: 0,
      y: 0,
    };

    return;
  }

  if (
    errorDistance >=
    RECONCILIATION_SNAP_DISTANCE
  ) {
    predictedPlayerPosition = {
      x: serverPlayer.position.x,
      y: serverPlayer.position.y,
      z: serverPlayer.position.z,
    };

    pendingPositionCorrection = {
      x: 0,
      y: 0,
    };

    writeLog(
      `Prediction snapped to server. error=${errorDistance.toFixed(3)}`,
    );

    return;
  }

  pendingPositionCorrection = {
    x: errorX,
    y: errorY,
  };
}

function removeAcknowledgedInputs(
  acknowledgedSequence: number,
): void {
  while (
    pendingInputs.length > 0
  ) {
    const pendingInput =
      pendingInputs[0];

    if (
      !isSequenceAcknowledged(
        pendingInput.sequence,
        acknowledgedSequence,
      )
    ) {
      break;
    }

    pendingInputs.shift();
  }
}

function isSequenceAcknowledged(
  sequence: number,
  acknowledgedSequence: number,
): boolean {
  if (
    sequence ===
    acknowledgedSequence
  ) {
    return true;
  }

  return isSequenceNewer(
    acknowledgedSequence,
    sequence,
  );
}

function isSequenceNewer(
  candidate: number,
  current: number,
): boolean {
  const difference =
    (candidate - current) >>> 0;

  return (
    difference !== 0 &&
    difference < 0x80000000
  );
}

function hasReachedTickInterval(
  currentTick: number,
  previousTick: number,
  intervalTicks: number,
): boolean {
  const elapsedTicks =
    (currentTick - previousTick) >>> 0;

  return elapsedTicks >= intervalTicks;
}

function normalizeMovement(
  movement: Vec2,
): Vec2 {
  if (
    !Number.isFinite(
      movement.x,
    ) ||
    !Number.isFinite(
      movement.y,
    )
  ) {
    return {
      x: 0,
      y: 0,
    };
  }

  const lengthSquared =
    movement.x *
      movement.x +
    movement.y *
      movement.y;

  if (lengthSquared <= 1) {
    return {
      x: movement.x,
      y: movement.y,
    };
  }

  const length =
    Math.sqrt(lengthSquared);

  return {
    x: movement.x / length,
    y: movement.y / length,
  };
}

function resetClientSimulationClock(): void {
  clientSimulationTick = 0;
  simulationAccumulatorSeconds = 0;

  lastFrameMilliseconds =
    performance.now();
}

function updateRendererState(): void {
  const rendererState:
    PixiRendererState = {
      serverTick: lastServerTick,
      playerEntityNetId,
      movement:
        inputController.getMovement(),
      predictedPlayerPosition,
      entities: entitiesByNetId,
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
    if (
      simulationFrameRequestId !== null
    ) {
      cancelAnimationFrame(
        simulationFrameRequestId,
      );
    }

    pixiRenderer.destroy();
    inputController.destroy();
    connection.disconnect();
  },
);