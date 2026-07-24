import { Container, Graphics, Text, TextStyle } from 'pixi.js';
import PixiApplication from './render/PixiApplication';
import RenderLoop from './render/RenderLoop';
import ResizeManager from './render/ResizeManager';
import DeviceRecovery from './render/DeviceRecovery';
import SceneManager from './render/scene/SceneManager';
import { CameraController } from './render/camera/CameraController';
import { CameraTransform } from './render/camera/CameraTransform';
import { BrowserInputAdapter } from './input/BrowserInputAdapter';
import { WasmBridge, RenderFrame, RenderSprite } from './bridge/WasmBridge';
import { TransportBridge } from './bridge/TransportBridge';
import { ChunkRenderer } from './render/ChunkRenderer';
import { RsiSpriteRenderer } from './render/RsiSpriteRenderer';

async function bootstrap() {
    let canvas = document.querySelector<HTMLCanvasElement>('#game');
    if (!canvas) {
        canvas = document.createElement('canvas');
        canvas.id = 'game';
        document.body.appendChild(canvas);
    }
    canvas.style.width = '100vw';
    canvas.style.height = '100vh';
    canvas.style.display = 'block';

    // 1. Init PixiJS Application
    const pixiApp = new PixiApplication();
    await pixiApp.init(canvas);
    const app = pixiApp.getApp();

    const resizeManager = new ResizeManager(app);
    resizeManager.init();

    const deviceRecovery = new DeviceRecovery(app);
    deviceRecovery.init();

    // 2. Scene setup
    const sceneManager = new SceneManager(app);

    // 3. Camera
    const cameraTransform = new CameraTransform(sceneManager.worldRoot);
    const cameraController = new CameraController(cameraTransform);

    // 4. Chunked Tile Map Renderer
    const chunkRenderer = new ChunkRenderer(sceneManager.tileLayer);
    chunkRenderer.initDefaultStationMap();

    // 5. HUD Elements
    const hudContainer = sceneManager.hudLayer;

    const titleStyle = new TextStyle({
        fontFamily: 'sans-serif',
        fontSize: 22,
        fontWeight: 'bold',
        fill: '#00f0ff',
        dropShadow: {
            alpha: 0.5,
            color: '#000000',
            blur: 4,
            distance: 2,
        },
    });

    const titleText = new Text({ text: 'HONKNET SS15 • WASM ClientRuntime + Server Replication Pipeline', style: titleStyle });
    titleText.position.set(20, 20);
    hudContainer.addChild(titleText);

    const statusStyle = new TextStyle({
        fontFamily: 'sans-serif',
        fontSize: 14,
        fill: '#94a3b8',
    });
    const statusText = new Text({
        text: 'Connecting to WebSocket server...',
        style: statusStyle,
    });
    statusText.position.set(20, 52);
    hudContainer.addChild(statusText);

    // 6. Input Adapter
    const inputAdapter = new BrowserInputAdapter();
    inputAdapter.attach(canvas);

    // 7. WASM & Network Transport Bridges
    const wasmBridge = new WasmBridge();
    const transportBridge = new TransportBridge();

    // Dynamically import WASM module if available
    try {
        // @ts-ignore
        const wasmPkg = await import('../pkg/honknet_web.js').catch(() => null);
        if (wasmPkg) {
            await wasmPkg.default();
            const runtime = new wasmPkg.WasmClientRuntime();
            wasmBridge.setRuntime(runtime);
            console.log('[WASM] WasmClientRuntime initialized');
        }
    } catch (e) {
        console.warn('[WASM] Direct module load skipped, using bridge fallback:', e);
    }

    transportBridge.setOnMessage((bytes: Uint8Array) => {
        wasmBridge.pushNetworkMessage(bytes);
    });

    const wsUrl = `ws://${window.location.hostname || '127.0.0.1'}:3015`;
    transportBridge.connect(wsUrl).then(() => {
        const helloPayload = wasmBridge.createHelloPayload();
        if (helloPayload) {
            transportBridge.send(helloPayload);
        }
    }).catch((err) => {
        console.error('[Transport] Connection failed:', err);
    });

    // 8. Dynamic Entity Graphics Container Map
    const rsiSpriteRenderer = new RsiSpriteRenderer();
    const entityGfxMap = new Map<number, Container>();

    // 9. Input & Render Loop
    const keysPressed: Record<string, boolean> = {};
    window.addEventListener('keydown', (e) => {
        keysPressed[e.code] = true;
        keysPressed[e.key.toLowerCase()] = true;
    });
    window.addEventListener('keyup', (e) => {
        keysPressed[e.code] = false;
        keysPressed[e.key.toLowerCase()] = false;
    });

    let inputSeq = 1;
    let actionSeq = 1;
    let actionMode:
        | 'interact' | 'attack' | 'pickup'
        | 'grab' | 'pull' | 'buckle' | 'store' | 'carry' = 'interact';
    let latestSprites: RenderSprite[] = [];
    let lastActionStatus = 'ready';
    let lobbyReady = false;
    const pendingActions = new Map<number, string>();
    let lastSentMoveX = 0;
    let lastSentMoveY = 0;
    let inputAccumulator = 0;
    const INPUT_TICK_RATE = 1.0 / 30.0; // Fixed 30 TPS Input Rate Limit

    const sendAction = (
        action:
            | 'interact' | 'attack' | 'pickup'
            | 'bandage' | 'bruise' | 'burn' | 'cpr'
            | 'surgeryChest'
            | 'grab' | 'releaseGrab' | 'pull' | 'stopPulling'
            | 'carry' | 'dropCarried'
            | 'buckle' | 'unbuckle'
            | 'equipJumpsuit' | 'unequipJumpsuit' | 'store' | 'drop',
        entityId: number | bigint = 0,
    ) => {
        const sequence = actionSeq++;
        const payload = wasmBridge.createActionPayload(sequence, action, entityId);
        if (!payload || payload.length === 0) {
            lastActionStatus = 'client unavailable';
            return;
        }
        pendingActions.set(sequence, action);
        transportBridge.send(payload);
        lastActionStatus = `${action} pending`;
    };

    canvas.addEventListener('click', (event) => {
        const bounds = canvas.getBoundingClientRect();
        const screenX = (event.clientX - bounds.left) * (app.screen.width / bounds.width);
        const screenY = (event.clientY - bounds.top) * (app.screen.height / bounds.height);
        const world = cameraTransform.screenToWorld(screenX, screenY);
        let nearest: RenderSprite | null = null;
        let nearestDistance = 24 / Math.max(cameraTransform.worldRoot.scale.x, 0.001);
        for (const sprite of latestSprites) {
            const distance = Math.hypot(sprite.x - world.x, sprite.y - world.y);
            if (distance <= nearestDistance) {
                nearest = sprite;
                nearestDistance = distance;
            }
        }
        if (nearest) {
            sendAction(event.shiftKey ? 'attack' : actionMode, nearest.entity_id);
        } else {
            lastActionStatus = 'no target';
        }
    });

    window.addEventListener('keydown', (event) => {
        if (event.code === 'Enter' && !event.repeat) {
            lobbyReady = !lobbyReady;
            const payload = wasmBridge.createLobbyReadyPayload(lobbyReady, 'medical_doctor');
            if (payload) transportBridge.send(payload);
        }
        if (event.code === 'Digit1') actionMode = 'interact';
        if (event.code === 'Digit2') actionMode = 'attack';
        if (event.code === 'Digit3') actionMode = 'pickup';
        if (event.code === 'Digit4') actionMode = 'grab';
        if (event.code === 'Digit5') actionMode = 'pull';
        if (event.code === 'Digit6') actionMode = 'buckle';
        if (event.code === 'Digit7') actionMode = 'store';
        if (event.code === 'Digit8') actionMode = 'carry';
        if (event.code === 'KeyQ' && !event.repeat) sendAction('drop');
        if (event.code === 'KeyE' && !event.repeat) sendAction('equipJumpsuit');
        if (event.code === 'KeyX' && !event.repeat) sendAction('unequipJumpsuit');
        if (event.code === 'KeyR' && !event.repeat) sendAction('releaseGrab');
        if (event.code === 'KeyT' && !event.repeat) sendAction('stopPulling');
        if (event.code === 'KeyY' && !event.repeat) sendAction('dropCarried');
        if (event.code === 'KeyU' && !event.repeat) sendAction('unbuckle');
        if (
            ['KeyB', 'KeyC', 'KeyV', 'KeyG', 'KeyH'].includes(event.code) &&
            !event.repeat
        ) {
            const bounds = canvas.getBoundingClientRect();
            const world = cameraTransform.screenToWorld(
                (inputAdapter.mouse.x - bounds.left) * (app.screen.width / bounds.width),
                (inputAdapter.mouse.y - bounds.top) * (app.screen.height / bounds.height),
            );
            const target = latestSprites
                .map((sprite) => ({
                    sprite,
                    distance: Math.hypot(sprite.x - world.x, sprite.y - world.y),
                }))
                .filter((candidate) => candidate.distance <= 24)
                .sort((left, right) => left.distance - right.distance)[0]?.sprite;
            if (target) {
                const medicalAction = {
                    KeyB: 'bandage',
                    KeyC: 'cpr',
                    KeyV: 'bruise',
                    KeyG: 'burn',
                    KeyH: 'surgeryChest',
                }[event.code] as 'bandage' | 'cpr' | 'bruise' | 'burn' | 'surgeryChest';
                sendAction(medicalAction, target.entity_id);
            } else {
                lastActionStatus = 'no medical target';
            }
        }
    });

    const loop = new RenderLoop();
    loop.addCallback((delta: number) => {
        inputAccumulator += delta;

        // Collect normalized movement vector
        let moveX = 0;
        let moveY = 0;

        if (keysPressed['KeyW'] || keysPressed['w'] || keysPressed['ц'] || keysPressed['ArrowUp']) moveY -= 1;
        if (keysPressed['KeyS'] || keysPressed['s'] || keysPressed['ы'] || keysPressed['ArrowDown']) moveY += 1;
        if (keysPressed['KeyA'] || keysPressed['a'] || keysPressed['ф'] || keysPressed['ArrowLeft']) moveX -= 1;
        if (keysPressed['KeyD'] || keysPressed['d'] || keysPressed['в'] || keysPressed['ArrowRight']) moveX += 1;

        if (moveX !== 0 || moveY !== 0) {
            const len = Math.hypot(moveX, moveY);
            moveX /= len;
            moveY /= len;
        }

        // Transmit input on fixed 30 TPS ticks or immediately on key release (stop)
        const isStopping = (moveX === 0 && moveY === 0 && (lastSentMoveX !== 0 || lastSentMoveY !== 0));
        if (inputAccumulator >= INPUT_TICK_RATE || isStopping) {
            if (isStopping || moveX !== 0 || moveY !== 0 || lastSentMoveX !== 0 || lastSentMoveY !== 0) {
                lastSentMoveX = moveX;
                lastSentMoveY = moveY;

                const seq = inputSeq++;
                wasmBridge.pushInput(seq, moveX, moveY);

                const inputPayload = wasmBridge.createInputPayload(seq, moveX, moveY);
                if (inputPayload) {
                    transportBridge.send(inputPayload);
                }
            }
            inputAccumulator %= INPUT_TICK_RATE;
        }

        // Tick WASM ClientRuntime
        wasmBridge.tickClient(delta);

        // Extract RenderFrame from WASM and update PixiJS entities
        const frame: RenderFrame | null = wasmBridge.extractRenderFrame();
        if (frame) {
            if (frame.tick) {
                const ackPayload = wasmBridge.createAckPayload(frame.tick);
                if (ackPayload) {
                    transportBridge.send(ackPayload);
                }
            }

            if (frame.tiles && frame.tiles.length > 0) {
                for (const tileUpdate of frame.tiles) {
                    chunkRenderer.updateChunk(tileUpdate);
                }
            }

            if (frame.sprites) {
                latestSprites = frame.sprites;
                const currentRenderIds = new Set<number>();

                for (const sprite of frame.sprites) {
                    currentRenderIds.add(sprite.render_id);

                    let entityContainer = entityGfxMap.get(sprite.render_id);
                    if (!entityContainer) {
                        entityContainer = rsiSpriteRenderer.createEntityContainer(sprite) as any;
                        sceneManager.entityLayer.addChild(entityContainer!);
                        entityGfxMap.set(sprite.render_id, entityContainer! as any);
                    }

                    rsiSpriteRenderer.updateEntityContainer(entityContainer!, sprite);
                }

                // Cleanup despawned / unrendered entities
                for (const [id, entityContainer] of entityGfxMap.entries()) {
                    if (!currentRenderIds.has(id)) {
                        sceneManager.entityLayer.removeChild(entityContainer);
                        entityContainer.destroy({ children: true });
                        entityGfxMap.delete(id);
                    }
                }
            }
    }

        for (const result of wasmBridge.drainActionResults()) {
            const action = pendingActions.get(result.sequence) ?? 'action';
            pendingActions.delete(result.sequence);
            lastActionStatus = `${action}: ${result.status}`;
        }

        // Update HUD Diagnostics
        const hud = wasmBridge.getHudState();
        const lobby = wasmBridge.getLobbyState();
        const medical = hud.medical;
        const medicalLine = medical
            ? `State: ${hud.mob_state ?? 'Unknown'} | Blood: ${(medical.blood_fraction * 100).toFixed(0)}% | O₂: ${(medical.oxygen_saturation * 100).toFixed(0)}% | Pain: ${medical.pain.toFixed(0)} | Shock: ${medical.shock.toFixed(0)}`
            : 'State: synchronizing';
        const interactionLine = hud.interaction
            ? `Grab: ${hud.interaction.grab_strength ?? 'none'} | Timed action: ${hud.interaction.action_kind ?? 'none'}`
            : 'Grab: none | Timed action: none';
        statusText.text = `Backend: WebGPU/WebGL2 | WASM: ${wasmBridge.isLoaded() ? 'Loaded' : 'Standalone'} | Server: ${wsUrl}\n` +
            `Diagnostics: ${wasmBridge.getDiagnostics()}\n` +
            `Round: ${lobby?.phase ?? 'Synchronizing'} #${lobby?.round_id ?? '-'} | Ready: ${lobby?.ready_players ?? 0}/${lobby?.connected_players ?? 0} | Job: ${lobby?.assigned_job ?? 'unassigned'} | Enter toggles ready\n` +
            `${medicalLine}\n${interactionLine}\n` +
            `Action: ${actionMode} | Result: ${lastActionStatus} | Pending: ${pendingActions.size}\n` +
            `Controls: WASD | 1 interact | 2 attack | 3 pickup | 4 grab | 5 pull | 6 buckle | 7 store | 8 carry\n` +
            `Inventory: E equip jumpsuit | X unequip | Q drop | R release | T stop pull | Y drop carried | U unbuckle\n` +
            `Medical: B bandage | V bruise pack | G burn gel | C CPR | H surgery step | Q drop`;

        cameraTransform.updateScreenSize(app.screen.width, app.screen.height);
        cameraController.update(delta);
        inputAdapter.endFrame();
    });

    loop.start();
}

bootstrap().catch(console.error);
