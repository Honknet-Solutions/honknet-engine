import './style.css';

type Vec2 = {
  x: number;
  y: number;
};

type NetPosition = {
  x: number;
  y: number;
  z: number;
};

type EntitySnapshot = {
  net_id: number;
  prototype: string;
  position: NetPosition;
};

type ClientMessage =
  | {
      type: 'Hello';
      data: {
        client_version: string;
        identity_id: string;
      };
    }
  | { type: 'Input'; data: { seq: number; movement: Vec2 } }
  | { type: 'Chat'; data: { text: string } }
  | { type: 'Interact'; data: { target: number } };

type ServerMessage =
  | {
      type: 'Welcome';
      data: {
        client_id: string;
        entity_net_id: number;
      };
    }
  | { type: 'Snapshot'; data: { tick: number; entities: EntitySnapshot[] } }
  | { type: 'Chat'; data: { from: string; text: string } }
  | { type: 'Error'; data: { message: string } };

const CLIENT_VERSION = '0.1.0-dev';
const DEFAULT_SERVER_URL = 'ws://127.0.0.1:3015';
const GUEST_IDENTITY_STORAGE_KEY = 'ss15.guestIdentityId';
const INPUT_SEND_INTERVAL_MS = 50;
const WORLD_SCALE = 32;

const app = document.querySelector<HTMLDivElement>('#app');

if (!app) {
  throw new Error('Missing #app root element');
}

app.innerHTML = `
  <main class="shell">
    <section class="panel">
      <p class="eyebrow">Honknet Solutions</p>
      <h1>Space Station 15</h1>
      <p>Browser-first modular 2D multiplayer immersive simulation framework.</p>

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
        Click the page after connecting, then use WASD or arrow keys.
      </p>

      <pre id="log">Client booted. Waiting for connection.</pre>
    </section>
  </main>
`;

const log = document.querySelector<HTMLPreElement>('#log');
const button = document.querySelector<HTMLButtonElement>('#connect');
const serverUrlInput = document.querySelector<HTMLInputElement>('#server-url');
const canvasElement = document.querySelector<HTMLCanvasElement>('#viewport');
const identityStatus = document.querySelector<HTMLElement>('#identity-status');
const clientStatus = document.querySelector<HTMLElement>('#client-status');
const entityStatus = document.querySelector<HTMLElement>('#entity-status');
const tickStatus = document.querySelector<HTMLElement>('#tick-status');

if (!canvasElement) {
  throw new Error('Missing #viewport canvas');
}

const canvasContextMaybe = canvasElement.getContext('2d');

if (!canvasContextMaybe) {
  throw new Error('Failed to create 2D canvas context');
}

const canvas: HTMLCanvasElement = canvasElement;
const canvasContext: CanvasRenderingContext2D = canvasContextMaybe;

let socket: WebSocket | null = null;
let clientId: string | null = null;
let playerEntityNetId: number | null = null;
let lastServerTick: number | null = null;
let inputSeq = 0;

const pressedKeys = new Set<string>();
const entitiesByNetId = new Map<number, EntitySnapshot>();
const playerIdentityId = getOrCreateGuestIdentityId();

setText(identityStatus, playerIdentityId);
writeLog(`Guest identity: ${playerIdentityId}`);

window.setInterval(() => {
  sendCurrentInput();
}, INPUT_SEND_INTERVAL_MS);

window.requestAnimationFrame(renderFrame);

window.addEventListener('keydown', (event: KeyboardEvent) => {
  if (isMovementKey(event.key)) {
    event.preventDefault();
    pressedKeys.add(event.key.toLowerCase());
  }
});

window.addEventListener('keyup', (event: KeyboardEvent) => {
  if (isMovementKey(event.key)) {
    event.preventDefault();
    pressedKeys.delete(event.key.toLowerCase());
  }
});

window.addEventListener('blur', () => {
  pressedKeys.clear();
});

document.addEventListener('visibilitychange', () => {
  if (document.hidden) {
    pressedKeys.clear();
  }
});

function getOrCreateGuestIdentityId(): string {
  const existingIdentityId = localStorage.getItem(GUEST_IDENTITY_STORAGE_KEY);

  if (existingIdentityId && existingIdentityId.trim().length > 0) {
    return existingIdentityId;
  }

  const newIdentityId = `guest-${crypto.randomUUID()}`;
  localStorage.setItem(GUEST_IDENTITY_STORAGE_KEY, newIdentityId);

  return newIdentityId;
}

function setText(element: HTMLElement | null, value: string): void {
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
  const nextLines = [...currentLines, `${new Date().toLocaleTimeString()} ${message}`];

  log.textContent = nextLines.slice(-18).join('\n');
}

function sendClientMessage(message: ClientMessage): void {
  if (!socket || socket.readyState !== WebSocket.OPEN) {
    return;
  }

  socket.send(JSON.stringify(message));
}

function isMovementKey(key: string): boolean {
  const normalizedKey = key.toLowerCase();

  return (
    normalizedKey === 'w' ||
    normalizedKey === 'a' ||
    normalizedKey === 's' ||
    normalizedKey === 'd' ||
    normalizedKey === 'arrowup' ||
    normalizedKey === 'arrowleft' ||
    normalizedKey === 'arrowdown' ||
    normalizedKey === 'arrowright'
  );
}

function getCurrentMovement(): Vec2 {
  let x = 0;
  let y = 0;

  if (pressedKeys.has('a') || pressedKeys.has('arrowleft')) {
    x -= 1;
  }

  if (pressedKeys.has('d') || pressedKeys.has('arrowright')) {
    x += 1;
  }

  if (pressedKeys.has('w') || pressedKeys.has('arrowup')) {
    y -= 1;
  }

  if (pressedKeys.has('s') || pressedKeys.has('arrowdown')) {
    y += 1;
  }

  return { x, y };
}

function sendCurrentInput(): void {
  if (!socket || socket.readyState !== WebSocket.OPEN || playerEntityNetId === null) {
    return;
  }

  const movement = getCurrentMovement();

  if (movement.x === 0 && movement.y === 0) {
    return;
  }

  inputSeq += 1;

  sendClientMessage({
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
        identity_id: playerIdentityId,
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
    setText(clientStatus, 'disconnected');
  });

  socket.addEventListener('error', () => {
    writeLog('WebSocket error. Is the Rust server running?');
  });
}

function renderFrame(): void {
  drawWorld();
  window.requestAnimationFrame(renderFrame);
}

function drawWorld(): void {
  canvasContext.clearRect(0, 0, canvas.width, canvas.height);

  drawGrid();

  for (const entity of entitiesByNetId.values()) {
    drawEntity(entity);
  }

  drawHud();
}

function drawGrid(): void {
  const centerX = canvas.width / 2;
  const centerY = canvas.height / 2;

  canvasContext.lineWidth = 1;
  canvasContext.strokeStyle = 'rgba(255, 255, 255, 0.08)';

  for (let x = centerX % WORLD_SCALE; x < canvas.width; x += WORLD_SCALE) {
    canvasContext.beginPath();
    canvasContext.moveTo(x, 0);
    canvasContext.lineTo(x, canvas.height);
    canvasContext.stroke();
  }

  for (let y = centerY % WORLD_SCALE; y < canvas.height; y += WORLD_SCALE) {
    canvasContext.beginPath();
    canvasContext.moveTo(0, y);
    canvasContext.lineTo(canvas.width, y);
    canvasContext.stroke();
  }

  canvasContext.strokeStyle = 'rgba(255, 255, 255, 0.3)';

  canvasContext.beginPath();
  canvasContext.moveTo(centerX, 0);
  canvasContext.lineTo(centerX, canvas.height);
  canvasContext.stroke();

  canvasContext.beginPath();
  canvasContext.moveTo(0, centerY);
  canvasContext.lineTo(canvas.width, centerY);
  canvasContext.stroke();
}

function drawEntity(entity: EntitySnapshot): void {
  const screenPosition = worldToScreen(entity.position);
  const isPlayer = entity.net_id === playerEntityNetId;

  canvasContext.beginPath();
  canvasContext.arc(screenPosition.x, screenPosition.y, isPlayer ? 12 : 10, 0, Math.PI * 2);
  canvasContext.fillStyle = isPlayer ? '#7cffc4' : '#ffcc66';
  canvasContext.fill();

  canvasContext.lineWidth = 2;
  canvasContext.strokeStyle = isPlayer ? '#eafff6' : '#fff0c2';
  canvasContext.stroke();

  canvasContext.fillStyle = '#ffffff';
  canvasContext.font = '12px monospace';
  canvasContext.textAlign = 'center';
  canvasContext.fillText(
    `${entity.net_id}${isPlayer ? ' YOU' : ''}`,
    screenPosition.x,
    screenPosition.y - 18,
  );
}

function drawHud(): void {
  canvasContext.fillStyle = 'rgba(0, 0, 0, 0.45)';
  canvasContext.fillRect(12, 12, 250, 72);

  canvasContext.fillStyle = '#ffffff';
  canvasContext.font = '13px monospace';
  canvasContext.textAlign = 'left';

  const movement = getCurrentMovement();

  canvasContext.fillText(`tick: ${lastServerTick ?? '-'}`, 24, 34);
  canvasContext.fillText(`entity: ${playerEntityNetId ?? '-'}`, 24, 54);
  canvasContext.fillText(`input: x=${movement.x}, y=${movement.y}`, 24, 74);
}

function worldToScreen(position: NetPosition): Vec2 {
  return {
    x: canvas.width / 2 + position.x * WORLD_SCALE,
    y: canvas.height / 2 + position.y * WORLD_SCALE,
  };
}

button?.addEventListener('click', () => {
  connectToServer();
  window.focus();
});