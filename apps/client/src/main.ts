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
import {
  PROTOCOL_VERSION,
  type EntityNetId,
  type MapSnapshot,
  type NetPosition,
  type ServerMessage,
  type Vec2,
} from './protocol';
import { TransformInterpolationSystem } from './systems/transformInterpolationSystem';

void bootstrap().catch((error: unknown) => {
  console.error('Client bootstrap failed:', error);
  const app = document.querySelector<HTMLDivElement>('#app');
  if (app) {
    app.innerHTML = `<main class="shell"><section class="panel"><h1>Client startup failed</h1><pre>${escapeHtml(String(error))}</pre></section></main>`;
  }
});

async function bootstrap(): Promise<void> {
  const CLIENT_VERSION = '0.1.0-rc.1';
  const DEFAULT_SERVER_URL = 'ws://127.0.0.1:3015';
  const app = requireElement<HTMLDivElement>('#app');

  app.innerHTML = `
    <main class="shell">
      <section class="panel">
        <header>
          <div><p class="eyebrow">Honknet Solutions</p><h1>Space Station 15</h1></div>
          <div class="connect-row"><input id="server-url" value="${DEFAULT_SERVER_URL}" /><button id="connect">Connect</button></div>
        </header>
        <div class="status-grid">
          <div><span>Identity</span><strong id="identity-status">-</strong></div>
          <div><span>Client</span><strong id="client-status">disconnected</strong></div>
          <div><span>Entity</span><strong id="entity-status">-</strong></div>
          <div><span>Server tick</span><strong id="tick-status">-</strong></div>
        </div>
        <div class="game-layout">
          <div><div id="viewport"></div><p class="hint">WASD / arrows — move. E — interact.</p></div>
          <aside>
            <h2>Inventory</h2><ul id="inventory"><li>Empty</li></ul>
            <h2>Chat</h2><div id="chat-log"></div>
            <form id="chat-form"><input id="chat-input" maxlength="500" autocomplete="off" placeholder="Message" /><button>Send</button></form>
          </aside>
        </div>
        <div id="ui-root"></div>
        <pre id="debug-log">Client booted.</pre>
      </section>
    </main>
  `;

  const viewport = requireElement<HTMLElement>('#viewport');
  const serverUrlInput = requireElement<HTMLInputElement>('#server-url');
  const connectButton = requireElement<HTMLButtonElement>('#connect');
  const identityStatus = requireElement<HTMLElement>('#identity-status');
  const clientStatus = requireElement<HTMLElement>('#client-status');
  const entityStatus = requireElement<HTMLElement>('#entity-status');
  const tickStatus = requireElement<HTMLElement>('#tick-status');
  const inventoryElement = requireElement<HTMLUListElement>('#inventory');
  const chatLog = requireElement<HTMLDivElement>('#chat-log');
  const chatForm = requireElement<HTMLFormElement>('#chat-form');
  const chatInput = requireElement<HTMLInputElement>('#chat-input');
  const debugLog = requireElement<HTMLPreElement>('#debug-log');
  const uiRoot = requireElement<HTMLDivElement>('#ui-root');

  const identityId = getOrCreateGuestIdentityId();
  const world = new ClientWorld();
  world.addSystem(new TransformInterpolationSystem());
  const renderer = new PixiRenderer(viewport);

  let map: MapSnapshot | null = null;
  let playerEntityNetId: EntityNetId | null = null;
  const uiSessions = new Map<string, HTMLElement>();
  let localState: LocalPlayerControllerState = {
    clientSimulationTick: 0,
    lastProcessedInputSeq: null,
    lastProcessedClientTick: null,
    predictedPlayerPosition: null,
    pendingInputCount: 0,
    predictionError: 0,
  };

  const connection = new ClientConnection({
    onOpen: () => {
      logDebug('Socket opened; sending Hello.');
      connection.send({
        type: 'Hello',
        data: {
          protocol_version: PROTOCOL_VERSION,
          client_version: CLIENT_VERSION,
          identity_id: identityId,
        },
      });
    },
    onMessage: handleServerMessage,
    onClose: () => {
      playerEntityNetId = null;
      map = null;
      world.clear();
      localController.clearPlayer();
      clientStatus.textContent = 'disconnected';
      entityStatus.textContent = '-';
      tickStatus.textContent = '-';
      updateInventory();
      closeAllUi();
      renderState();
    },
    onError: logDebug,
  });

  const input = new InputController({ onInteract: interactWithNearest });
  const localController = new LocalPlayerController({
    getMovement: () => input.getMovement(),
    isConnected: () => connection.isConnected,
    sendMessage: (message) => connection.send(message),
    resolveMovement: resolveLocalMovement,
    onFrame: (state) => {
      localState = state;
      renderState();
    },
    onPredictionSnap: (distance) => logDebug(`Prediction snap: ${distance.toFixed(3)}`),
  });

  await renderer.initialize();
  localController.start();
  identityStatus.textContent = identityId;

  let lastFrame = performance.now();
  let frameId = requestAnimationFrame(updateFrame);

  connectButton.addEventListener('click', () => {
    const url = serverUrlInput.value.trim() || DEFAULT_SERVER_URL;
    if (!connection.connect(url)) {
      logDebug('Connection is already active.');
      return;
    }
    clientStatus.textContent = 'connecting';
  });

  chatForm.addEventListener('submit', (event) => {
    event.preventDefault();
    const text = chatInput.value.trim();
    if (text && connection.send({ type: 'Chat', data: { text } })) {
      chatInput.value = '';
    }
  });

  window.addEventListener('beforeunload', () => {
    cancelAnimationFrame(frameId);
    localController.stop();
    input.destroy();
    renderer.destroy();
    connection.disconnect();
    world.clear();
  });

  function updateFrame(now: number): void {
    const delta = Math.min(Math.max((now - lastFrame) / 1000, 0), 0.1);
    lastFrame = now;
    world.update(delta);
    renderState();
    frameId = requestAnimationFrame(updateFrame);
  }

  function handleServerMessage(message: ServerMessage): void {
    switch (message.type) {
      case 'Welcome':
        if (message.data.protocol_version !== PROTOCOL_VERSION) {
          throw new Error(`Protocol mismatch: server=${message.data.protocol_version}, client=${PROTOCOL_VERSION}`);
        }
        map = message.data.map;
        playerEntityNetId = message.data.entity_net_id;
        world.clear();
        localController.setPlayerEntity(playerEntityNetId);
        clientStatus.textContent = message.data.client_id;
        entityStatus.textContent = String(playerEntityNetId);
        addChat('system', 'Connected to the server.');
        break;

      case 'Snapshot': {
        const result = world.applySnapshot(message.data.tick, message.data.entities);
        tickStatus.textContent = String(message.data.tick);
        const player = playerEntityNetId === null ? undefined : world.getEntity(playerEntityNetId);
        localController.handleSnapshot(
          player,
          message.data.last_processed_input_seq,
          message.data.last_processed_client_tick,
        );
        connection.send({ type: 'SnapshotAck', data: { tick: message.data.tick } });
        updateInventory();
        logDebug(
          `tick=${message.data.tick} entities=${message.data.entities.length} ` +
          `created=${result.created} updated=${result.updated} removed=${result.removed} ` +
          `error=${localState.predictionError.toFixed(4)}`,
        );
        break;
      }

      case 'StateDelta': {
        const result = world.applyDelta(
          message.data.tick,
          message.data.spawns,
          message.data.updates,
          message.data.despawns,
        );
        tickStatus.textContent = String(message.data.tick);
        const player = playerEntityNetId === null ? undefined : world.getEntity(playerEntityNetId);
        localController.handleSnapshot(
          player,
          message.data.last_processed_input_seq,
          message.data.last_processed_client_tick,
        );
        connection.send({ type: 'SnapshotAck', data: { tick: message.data.tick } });
        updateInventory();
        if (result.created || result.updated || result.removed) {
          logDebug(
            `delta=${message.data.tick} spawn=${result.created} ` +
            `update=${result.updated} despawn=${result.removed}`,
          );
        }
        break;
      }

      case 'Chat':
        addChat(message.data.from, message.data.text);
        break;
      case 'System':
        addChat('system', message.data.text);
        break;
      case 'UiOpen':
        openUi(message.data.session_id, message.data.key, message.data.state);
        break;
      case 'UiState':
        updateUi(message.data.session_id, message.data.state);
        break;
      case 'UiClose':
        closeUi(message.data.session_id);
        break;
      case 'PlaySound':
        void new Audio(message.data.path).play().catch((error) => logDebug(`Audio error: ${String(error)}`));
        break;
      case 'Error':
        addChat('error', message.data.message);
        break;
      default: {
        const exhaustive: never = message;
        throw new Error(`Unhandled server message: ${JSON.stringify(exhaustive)}`);
      }
    }
    renderState();
  }

  function resolveLocalMovement(
    position: NetPosition,
    movement: Vec2,
    distance: number,
  ): NetPosition {
    const radius = 0.32;
    let nextX = position.x;
    let nextY = position.y;

    const candidateX =
      nextX + movement.x * distance;

    if (
      !isLocalPositionBlocked(
        candidateX,
        nextY,
        position.z,
        radius,
      )
    ) {
      nextX = candidateX;
    }

    const candidateY =
      nextY + movement.y * distance;

    if (
      !isLocalPositionBlocked(
        nextX,
        candidateY,
        position.z,
        radius,
      )
    ) {
      nextY = candidateY;
    }

    return {
      x: nextX,
      y: nextY,
      z: position.z,
    };
  }

  function isLocalPositionBlocked(
    x: number,
    y: number,
    z: number,
    radius: number,
  ): boolean {
    if (mapCircleCollides(x, y, radius)) {
      return true;
    }

    for (const [netId, entity] of world.getEntities()) {
      if (
        netId === playerEntityNetId ||
        !entity.door ||
        entity.door.open ||
        entity.position.z !== z
      ) {
        continue;
      }

      const nearestX = Math.max(
        entity.position.x - 0.45,
        Math.min(x, entity.position.x + 0.45),
      );
      const nearestY = Math.max(
        entity.position.y - 0.45,
        Math.min(y, entity.position.y + 0.45),
      );
      const deltaX = x - nearestX;
      const deltaY = y - nearestY;

      if (
        deltaX * deltaX + deltaY * deltaY <
        radius * radius
      ) {
        return true;
      }
    }

    return false;
  }

  function mapCircleCollides(
    x: number,
    y: number,
    radius: number,
  ): boolean {
    if (!map) {
      return false;
    }

    const minX = Math.floor(x - radius);
    const maxX = Math.floor(x + radius);
    const minY = Math.floor(y - radius);
    const maxY = Math.floor(y + radius);

    for (let tileY = minY; tileY <= maxY; tileY += 1) {
      for (let tileX = minX; tileX <= maxX; tileX += 1) {
        if (!isWallTile(tileX, tileY)) {
          continue;
        }

        const nearestX = Math.max(
          tileX,
          Math.min(x, tileX + 1),
        );
        const nearestY = Math.max(
          tileY,
          Math.min(y, tileY + 1),
        );
        const deltaX = x - nearestX;
        const deltaY = y - nearestY;

        if (
          deltaX * deltaX + deltaY * deltaY <
          radius * radius
        ) {
          return true;
        }
      }
    }

    return false;
  }

  function isWallTile(x: number, y: number): boolean {
    if (!map) {
      return false;
    }

    if (
      x < 0 ||
      y < 0 ||
      x >= map.width ||
      y >= map.height
    ) {
      return true;
    }

    return map.tiles[y * map.width + x] === 1;
  }

  function interactWithNearest(): void {
    if (playerEntityNetId === null || !connection.isConnected) return;
    const player = world.getEntity(playerEntityNetId);
    const origin = localState.predictedPlayerPosition ?? player?.position;
    if (!origin) return;

    let nearest: { id: EntityNetId; distance: number } | null = null;
    for (const [netId, entity] of world.getEntities()) {
      if (netId === playerEntityNetId || (!entity.door && !entity.item)) continue;
      const distance = Math.hypot(origin.x - entity.position.x, origin.y - entity.position.y);
      if (distance <= 1.75 && (!nearest || distance < nearest.distance)) {
        nearest = { id: netId, distance };
      }
    }

    if (nearest) {
      connection.send({ type: 'Interact', data: { target: nearest.id } });
    } else {
      addChat('system', 'Nothing nearby to interact with.');
    }
  }

  function updateInventory(): void {
    const inventory = playerEntityNetId === null ? undefined : world.getEntity(playerEntityNetId)?.inventory;
    inventoryElement.replaceChildren();
    if (!inventory || inventory.items.length === 0) {
      const item = document.createElement('li');
      item.textContent = 'Empty';
      inventoryElement.appendChild(item);
      return;
    }
    for (const name of inventory.items) {
      const item = document.createElement('li');
      item.textContent = name;
      inventoryElement.appendChild(item);
    }
  }

  function renderState(): void {
    const state: PixiRendererState = {
      map,
      playerEntityNetId,
      predictedPlayerPosition: localState.predictedPlayerPosition,
      entities: world.getEntities(),
    };
    renderer.update(state);
  }

  function addChat(from: string, text: string): void {
    const line = document.createElement('div');
    const name = document.createElement('strong');
    name.textContent = `${from}: `;
    line.append(name, document.createTextNode(text));
    chatLog.appendChild(line);
    chatLog.scrollTop = chatLog.scrollHeight;
  }

  function openUi(sessionId: string, key: string, state: unknown): void {
    closeUi(sessionId);
    const window = document.createElement('section');
    window.className = 'hui-window';
    window.dataset.sessionId = sessionId;
    const title = document.createElement('h2');
    title.textContent = key;
    const body = document.createElement('pre');
    body.textContent = JSON.stringify(state, null, 2);
    const close = document.createElement('button');
    close.textContent = 'Close';
    close.addEventListener('click', () => closeUi(sessionId));
    window.append(title, body, close);
    uiRoot.appendChild(window);
    uiSessions.set(sessionId, window);
  }

  function updateUi(sessionId: string, state: unknown): void {
    const window = uiSessions.get(sessionId);
    const body = window?.querySelector('pre');
    if (body) body.textContent = JSON.stringify(state, null, 2);
  }

  function closeUi(sessionId: string): void {
    uiSessions.get(sessionId)?.remove();
    uiSessions.delete(sessionId);
  }

  function closeAllUi(): void {
    for (const sessionId of [...uiSessions.keys()]) closeUi(sessionId);
  }

  function logDebug(message: string): void {
    const lines = debugLog.textContent?.split('\n') ?? [];
    debugLog.textContent = [...lines, `${new Date().toLocaleTimeString()} ${message}`]
      .slice(-12)
      .join('\n');
  }
}

function requireElement<T extends Element>(selector: string): T {
  const element = document.querySelector<T>(selector);
  if (!element) throw new Error(`Missing element ${selector}`);
  return element;
}

function escapeHtml(value: string): string {
  return value
    .replaceAll('&', '&amp;')
    .replaceAll('<', '&lt;')
    .replaceAll('>', '&gt;')
    .replaceAll('"', '&quot;')
    .replaceAll("'", '&#039;');
}
