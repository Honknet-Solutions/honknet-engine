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
  MapSnapshot,
  ServerMessage,
} from './protocol';
import { TransformInterpolationSystem } from './systems/transformInterpolationSystem';

const CLIENT_VERSION = '0.2.0-vertical-slice';
const DEFAULT_SERVER_URL = 'ws://127.0.0.1:3015';

void bootstrap().catch((error: unknown) => {
  console.error('Client bootstrap failed:', error);

  const root =
    document.querySelector<HTMLDivElement>('#app');

  if (root) {
    root.innerHTML = `
      <main class="shell">
        <section class="panel">
          <h1>Client startup failed</h1>
          <pre>${escapeHtml(String(error))}</pre>
        </section>
      </main>
    `;
  }
});

async function bootstrap(): Promise<void> {
  const app =
    document.querySelector<HTMLDivElement>('#app');

  if (!app) {
    throw new Error('Missing #app');
  }

  app.innerHTML = `
    <main class="shell">
      <section class="panel">
        <header>
          <div>
            <p class="eyebrow">Honknet Solutions</p>
            <h1>Space Station 15</h1>
          </div>

          <div class="connect-row">
            <input
              id="server-url"
              value="${DEFAULT_SERVER_URL}"
            />

            <button id="connect">
              Connect
            </button>
          </div>
        </header>

        <div class="status-grid">
          <div>
            <span>Identity</span>
            <strong id="identity-status">-</strong>
          </div>

          <div>
            <span>Client</span>
            <strong id="client-status">
              disconnected
            </strong>
          </div>

          <div>
            <span>Entity</span>
            <strong id="entity-status">-</strong>
          </div>

          <div>
            <span>Server tick</span>
            <strong id="tick-status">-</strong>
          </div>
        </div>

        <div class="game-layout">
          <div>
            <div id="viewport"></div>

            <p class="hint">
              WASD / arrows — move. E — interact.
            </p>
          </div>

          <aside>
            <h2>Inventory</h2>

            <ul id="inventory">
              <li>Empty</li>
            </ul>

            <h2>Chat</h2>

            <div id="chat-log"></div>

            <form id="chat-form">
              <input
                id="chat-input"
                maxlength="500"
                autocomplete="off"
                placeholder="Message"
              />

              <button>Send</button>
            </form>
          </aside>
        </div>

        <pre id="debug-log">Client booted.</pre>
      </section>
    </main>
  `;

  const viewport =
    requireElement<HTMLElement>(
      '#viewport',
    );

  const serverUrlInput =
    requireElement<HTMLInputElement>(
      '#server-url',
    );

  const connectButton =
    requireElement<HTMLButtonElement>(
      '#connect',
    );

  const identityStatus =
    requireElement<HTMLElement>(
      '#identity-status',
    );

  const clientStatus =
    requireElement<HTMLElement>(
      '#client-status',
    );

  const entityStatus =
    requireElement<HTMLElement>(
      '#entity-status',
    );

  const tickStatus =
    requireElement<HTMLElement>(
      '#tick-status',
    );

  const inventoryElement =
    requireElement<HTMLUListElement>(
      '#inventory',
    );

  const chatLog =
    requireElement<HTMLDivElement>(
      '#chat-log',
    );

  const chatForm =
    requireElement<HTMLFormElement>(
      '#chat-form',
    );

  const chatInput =
    requireElement<HTMLInputElement>(
      '#chat-input',
    );

  const debugLog =
    requireElement<HTMLPreElement>(
      '#debug-log',
    );

  const identityId =
    getOrCreateGuestIdentityId();

  const world =
    new ClientWorld();

  world.addSystem(
    new TransformInterpolationSystem(),
  );

  const renderer =
    new PixiRenderer(viewport);

  let map: MapSnapshot | null = null;

  let playerEntityNetId:
    EntityNetId | null = null;

  let localState:
    LocalPlayerControllerState = {
      clientSimulationTick: 0,
      lastProcessedInputSeq: null,
      lastProcessedClientTick: null,
      predictedPlayerPosition: null,
      pendingInputCount: 0,
    };

  const connection =
    new ClientConnection({
      onOpen: () => {
        logDebug(
          'Socket opened; sending Hello.',
        );

        connection.send({
          type: 'Hello',
          data: {
            client_version:
              CLIENT_VERSION,

            identity_id:
              identityId,
          },
        });
      },

      onMessage:
        handleServerMessage,

      onClose: () => {
        playerEntityNetId = null;
        map = null;

        world.clear();
        localController.clearPlayer();

        clientStatus.textContent =
          'disconnected';

        entityStatus.textContent = '-';
        tickStatus.textContent = '-';

        updateInventory();
        renderState();
      },

      onError:
        logDebug,
    });

  const input =
    new InputController({
      onInteract: () => {
        interactWithNearest();
      },
    });

  const localController =
    new LocalPlayerController({
      getMovement: () =>
        input.getMovement(),

      isConnected: () =>
        connection.isConnected,

      sendMessage: (message) =>
        connection.send(message),

      onFrame: (state) => {
        localState = state;
        renderState();
      },

      onPredictionSnap: (
        distance,
      ) => {
        logDebug(
          `Prediction snap: ${distance.toFixed(3)}`,
        );
      },
    });

  await renderer.initialize();

  localController.start();

  identityStatus.textContent =
    identityId;

  let lastFrame =
    performance.now();

  let frameId =
    requestAnimationFrame(
      updateFrame,
    );

  connectButton.addEventListener(
    'click',
    () => {
      const url =
        serverUrlInput.value.trim() ||
        DEFAULT_SERVER_URL;

      if (!connection.connect(url)) {
        logDebug(
          'Connection is already active.',
        );

        return;
      }

      clientStatus.textContent =
        'connecting';
    },
  );

  chatForm.addEventListener(
    'submit',
    (event) => {
      event.preventDefault();

      const text =
        chatInput.value.trim();

      if (
        !text ||
        !connection.send({
          type: 'Chat',
          data: {
            text,
          },
        })
      ) {
        return;
      }

      chatInput.value = '';
    },
  );

  window.addEventListener(
    'beforeunload',
    () => {
      cancelAnimationFrame(frameId);

      localController.stop();
      input.destroy();
      renderer.destroy();
      connection.disconnect();
      world.clear();
    },
  );

  function updateFrame(
    now: number,
  ): void {
    const delta =
      Math.min(
        Math.max(
          (
            now -
            lastFrame
          ) / 1000,
          0,
        ),
        0.1,
      );

    lastFrame = now;

    world.update(delta);
    renderState();

    frameId =
      requestAnimationFrame(
        updateFrame,
      );
  }

  function handleServerMessage(
    message: ServerMessage,
  ): void {
    switch (message.type) {
      case 'Welcome':
        map =
          message.data.map;

        playerEntityNetId =
          message.data.entity_net_id;

        world.clear();

        localController.setPlayerEntity(
          playerEntityNetId,
        );

        clientStatus.textContent =
          message.data.client_id;

        entityStatus.textContent =
          String(playerEntityNetId);

        addChat(
          'system',
          'Connected to the server.',
        );

        break;

      case 'Snapshot': {
        const result =
          world.applySnapshot(
            message.data.tick,
            message.data.entities,
          );

        tickStatus.textContent =
          String(message.data.tick);

        const player =
          playerEntityNetId === null
            ? undefined
            : world.getEntity(
                playerEntityNetId,
              );

        localController.handleSnapshot(
          player,
          message.data
            .last_processed_input_seq,
          message.data
            .last_processed_client_tick,
        );

        updateInventory();

        logDebug(
          `tick=${message.data.tick} entities=${message.data.entities.length} ` +
            `created=${result.created} updated=${result.updated} removed=${result.removed}`,
        );

        break;
      }

      case 'Chat':
        addChat(
          message.data.from,
          message.data.text,
        );

        break;

      case 'System':
        addChat(
          'system',
          message.data.text,
        );

        break;

      case 'Error':
        addChat(
          'error',
          message.data.message,
        );

        break;

      default: {
        const unreachable: never =
          message;

        throw new Error(
          `Unhandled server message: ${JSON.stringify(unreachable)}`,
        );
      }
    }

    renderState();
  }

  function interactWithNearest(): void {
    if (
      playerEntityNetId === null ||
      !connection.isConnected
    ) {
      return;
    }

    const player =
      world.getEntity(
        playerEntityNetId,
      );

    const origin =
      localState
        .predictedPlayerPosition ??
      player?.position;

    if (!origin) {
      return;
    }

    let nearest: {
      id: EntityNetId;
      distance: number;
    } | null = null;

    for (
      const [
        netId,
        entity,
      ] of world.getEntities()
    ) {
      if (
        netId ===
          playerEntityNetId ||
        (
          !entity.door &&
          !entity.item
        )
      ) {
        continue;
      }

      const distance =
        Math.hypot(
          origin.x -
            entity.position.x,

          origin.y -
            entity.position.y,
        );

      if (
        distance <= 1.75 &&
        (
          nearest === null ||
          distance <
            nearest.distance
        )
      ) {
        nearest = {
          id: netId,
          distance,
        };
      }
    }

    if (nearest) {
      connection.send({
        type: 'Interact',
        data: {
          target:
            nearest.id,
        },
      });

      return;
    }

    addChat(
      'system',
      'Nothing nearby to interact with.',
    );
  }

  function updateInventory(): void {
    const inventory =
      playerEntityNetId === null
        ? undefined
        : world.getEntity(
            playerEntityNetId,
          )?.inventory;

    inventoryElement.replaceChildren();

    if (
      !inventory ||
      inventory.items.length === 0
    ) {
      const item =
        document.createElement('li');

      item.textContent = 'Empty';

      inventoryElement.appendChild(
        item,
      );

      return;
    }

    for (
      const name
      of inventory.items
    ) {
      const item =
        document.createElement('li');

      item.textContent = name;

      inventoryElement.appendChild(
        item,
      );
    }
  }

  function renderState(): void {
    const state:
      PixiRendererState = {
        map,

        playerEntityNetId,

        predictedPlayerPosition:
          localState
            .predictedPlayerPosition,

        entities:
          world.getEntities(),
      };

    renderer.update(state);
  }

  function addChat(
    from: string,
    text: string,
  ): void {
    const line =
      document.createElement('div');

    const name =
      document.createElement(
        'strong',
      );

    name.textContent =
      `${from}: `;

    line.append(
      name,
      document.createTextNode(
        text,
      ),
    );

    chatLog.appendChild(line);

    chatLog.scrollTop =
      chatLog.scrollHeight;
  }

  function logDebug(
    message: string,
  ): void {
    const lines =
      debugLog.textContent?.split(
        '\n',
      ) ?? [];

    debugLog.textContent = [
      ...lines,

      `${new Date().toLocaleTimeString()} ${message}`,
    ]
      .slice(-10)
      .join('\n');
  }
}

function requireElement<
  TElement extends Element,
>(
  selector: string,
): TElement {
  const element =
    document.querySelector<TElement>(
      selector,
    );

  if (!element) {
    throw new Error(
      `Missing element ${selector}`,
    );
  }

  return element;
}

function escapeHtml(
  value: string,
): string {
  return value
    .replaceAll('&', '&amp;')
    .replaceAll('<', '&lt;')
    .replaceAll('>', '&gt;')
    .replaceAll('"', '&quot;')
    .replaceAll("'", '&#039;');
}