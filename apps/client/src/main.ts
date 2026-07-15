import './style.css';

import { ClientConnection } from './connection';
import { ClientWorld } from './clientWorld';
import { getAuthToken, getOrCreateGuestIdentityId } from './identity';
import { InputController } from './input';
import { LocalizationBundle } from './localization';
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
import { renderHui, type HuiNode } from './ui/hui';
import { HuiRegistry } from './ui/huiRegistry';

void bootstrap().catch((error: unknown) => {
  console.error('Client bootstrap failed:', error);
  const app = document.querySelector<HTMLDivElement>('#app');
  if (app) {
    app.innerHTML = `<main class="shell"><section class="panel"><h1>Client startup failed</h1><pre>${escapeHtml(String(error))}</pre></section></main>`;
  }
});

async function bootstrap(): Promise<void> {
  const CLIENT_VERSION = '0.2.0-rc.1';
  const DEFAULT_SERVER_URL = 'ws://127.0.0.1:3015';
  const app = requireElement<HTMLDivElement>('#app');

  app.innerHTML = `
    <main class="shell">
      <section class="panel">
        <header>
          <div><p class="eyebrow">Honknet Solutions</p><h1>Honknet Runtime</h1></div>
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
  const localization = new LocalizationBundle();
  const huiRegistry = new HuiRegistry();
  const uiSessions = new Map<string, { key: string; state: Record<string, unknown>; root: HTMLElement; document: HuiNode }>();
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
          auth_token: getAuthToken(),
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

  await Promise.all([localization.initialize(), huiRegistry.initialize()]);
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
        tickStatus.textContent = String(message.data.server_tick);
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
          message.data.baseline_tick,
          message.data.spawns,
          message.data.updates,
          message.data.despawns,
        );
        if (!result.baselineMatched) {
          logDebug(
            `Delta baseline mismatch: local=${world.getServerTick()} server=${message.data.baseline_tick}; requesting full state.`,
          );
          connection.send({ type: 'RequestFullState' });
          break;
        }
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
            `delta=${message.data.tick} baseline=${message.data.baseline_tick} spawn=${result.created} ` +
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
        void openUi(message.data.session_id, message.data.key, message.data.state);
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
      case 'Pong':
        logDebug(`Pong ${message.data.nonce} at server tick ${message.data.server_tick}`);
        break;
      case 'Error':
        addChat('error', `[${message.data.code}] ${message.data.message}`);
        if (message.data.fatal) connection.disconnect();
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

      // Mirrors the authoritative server's circle-vs-circle collider test.
      const deltaX = x - entity.position.x;
      const deltaY = y - entity.position.y;
      const combinedRadius = radius + 0.45;

      if (
        deltaX * deltaX + deltaY * deltaY <
        combinedRadius * combinedRadius
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
    if (!map) return false;
    const playerGrid = playerEntityNetId === null
      ? null
      : world.getEntity(playerEntityNetId)?.gridId ?? null;
    const grids = playerGrid
      ? map.grids.filter((grid) => grid.id === playerGrid)
      : map.grids;

    for (const grid of grids) {
      const translatedX = x - grid.position[0];
      const translatedY = y - grid.position[1];
      const sin = Math.sin(-grid.rotation);
      const cos = Math.cos(-grid.rotation);
      const localX = translatedX * cos - translatedY * sin;
      const localY = translatedX * sin + translatedY * cos;
      const minX = Math.floor(localX - radius);
      const maxX = Math.floor(localX + radius);
      const minY = Math.floor(localY - radius);
      const maxY = Math.floor(localY + radius);

      for (let tileY = minY; tileY <= maxY; tileY += 1) {
        for (let tileX = minX; tileX <= maxX; tileX += 1) {
          const tileIndex = getGridTile(grid, tileX, tileY);
          if (tileIndex === null || !map.tile_definitions[tileIndex]?.solid) continue;
          const nearestX = Math.max(tileX, Math.min(localX, tileX + 1));
          const nearestY = Math.max(tileY, Math.min(localY, tileY + 1));
          const deltaX = localX - nearestX;
          const deltaY = localY - nearestY;
          if (deltaX * deltaX + deltaY * deltaY < radius * radius) {
            return true;
          }
        }
      }
    }
    return false;
  }

  function getGridTile(
    grid: MapSnapshot['grids'][number],
    x: number,
    y: number,
  ): number | null {
    for (const chunk of grid.chunks) {
      const localX = x - chunk.position[0];
      const localY = y - chunk.position[1];
      if (localX < 0 || localY < 0 || localX >= chunk.width || localY >= chunk.height) {
        continue;
      }
      return chunk.tiles[localY * chunk.width + localX] ?? null;
    }
    return null;
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
    for (const entry of inventory.items) {
      const item = document.createElement('li');
      item.textContent = entry.display_name;
      item.dataset.entityNetId = String(entry.entity_net_id);
      item.title = entry.prototype;
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

  async function openUi(sessionId: string, key: string, state: unknown): Promise<void> {
    closeUi(sessionId);
    try {
      const documentValue = await huiRegistry.load(key);
      const stateRecord = isRecord(state) ? structuredClone(state) : { value: state };
      const root = renderUiDocument(sessionId, key, documentValue, stateRecord);
      uiRoot.appendChild(root);
      uiSessions.set(sessionId, {
        key,
        state: stateRecord,
        root,
        document: documentValue,
      });
    } catch (error) {
      addChat('error', `Failed to open UI ${key}: ${String(error)}`);
    }
  }

  function renderUiDocument(
    sessionId: string,
    key: string,
    documentValue: HuiNode,
    state: Record<string, unknown>,
  ): HTMLElement {
    const root = renderHui(documentValue, {
      state,
      localize: (localizationKey) => localization.get(localizationKey),
      action: (action, payload) => {
        connection.send({
          type: 'UiAction',
          data: { session_id: sessionId, action, payload },
        });
      },
      sendMessage: (message, payload) => {
        connection.send({
          type: 'UiAction',
          data: { session_id: sessionId, action: message, payload },
        });
      },
      setState: (path, value) => {
        setStatePath(state, path, value);
        rerenderUi(sessionId);
      },
      closeWindow: () => {
        connection.send({
          type: 'UiAction',
          data: { session_id: sessionId, action: 'close', payload: null },
        });
        closeUi(sessionId);
      },
      playSound: (source) => {
        void new Audio(source).play().catch((error) => logDebug(`Audio error: ${String(error)}`));
      },
      resolveResource: (source) => source,
    });
    root.dataset.sessionId = sessionId;
    root.dataset.huiKey = key;
    return root;
  }

  function updateUi(sessionId: string, state: unknown): void {
    const session = uiSessions.get(sessionId);
    if (!session) return;
    session.state = isRecord(state) ? structuredClone(state) : { value: state };
    rerenderUi(sessionId);
  }

  function rerenderUi(sessionId: string): void {
    const session = uiSessions.get(sessionId);
    if (!session) return;
    const replacement = renderUiDocument(
      sessionId,
      session.key,
      session.document,
      session.state,
    );
    session.root.replaceWith(replacement);
    session.root = replacement;
  }

  function closeUi(sessionId: string): void {
    uiSessions.get(sessionId)?.root.remove();
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


function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

function setStatePath(target: Record<string, unknown>, path: string, value: unknown): void {
  const segments = path.replace(/^\$state\./, '').split('.').filter(Boolean);
  if (segments.length === 0) return;
  let cursor = target;
  for (const segment of segments.slice(0, -1)) {
    const existing = cursor[segment];
    if (!isRecord(existing)) cursor[segment] = {};
    cursor = cursor[segment] as Record<string, unknown>;
  }
  const finalSegment = segments.at(-1);
  if (finalSegment) cursor[finalSegment] = value;
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
