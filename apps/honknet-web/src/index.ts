import { Graphics, Text, TextStyle } from 'pixi.js';
import PixiApplication from './render/PixiApplication';
import RenderLoop from './render/RenderLoop';
import ResizeManager from './render/ResizeManager';
import DeviceRecovery from './render/DeviceRecovery';
import SceneManager from './render/scene/SceneManager';
import { CameraController } from './render/camera/CameraController';
import { CameraTransform } from './render/camera/CameraTransform';
import { BrowserInputAdapter } from './input/BrowserInputAdapter';
import { WasmBridge, RenderFrame } from './bridge/WasmBridge';
import { TransportBridge } from './bridge/TransportBridge';
import { ChunkRenderer } from './render/ChunkRenderer';

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
    const entityGfxMap = new Map<number, Graphics>();

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
    let lastSentMoveX = 0;
    let lastSentMoveY = 0;

    const loop = new RenderLoop();
    loop.addCallback((delta: number) => {
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

        // Transmit input when moving or when stopping (WASD key release)
        if (moveX !== lastSentMoveX || moveY !== lastSentMoveY || moveX !== 0 || moveY !== 0) {
            lastSentMoveX = moveX;
            lastSentMoveY = moveY;

            const seq = inputSeq++;
            wasmBridge.pushInput(seq, moveX, moveY);

            const inputPayload = wasmBridge.createInputPayload(seq, moveX, moveY);
            if (inputPayload) {
                transportBridge.send(inputPayload);
            }
        }

        // Tick WASM ClientRuntime
        wasmBridge.tickClient(delta);

        // Extract RenderFrame from WASM and update PixiJS entities
        const frame: RenderFrame | null = wasmBridge.extractRenderFrame();
        if (frame) {
            if (frame.tiles && frame.tiles.length > 0) {
                for (const tileUpdate of frame.tiles) {
                    chunkRenderer.updateChunk(tileUpdate);
                }
            }

            if (frame.sprites) {
                const currentRenderIds = new Set<number>();

            for (const sprite of frame.sprites) {
                currentRenderIds.add(sprite.render_id);

                let gfx = entityGfxMap.get(sprite.render_id);
                if (!gfx) {
                    gfx = new Graphics();
                    gfx.circle(0, 0, 20);
                    gfx.fill({ color: sprite.color || 0x00f0ff });
                    gfx.stroke({ color: 0xffffff, width: 3 });

                    gfx.moveTo(0, 0);
                    gfx.lineTo(30, 0);
                    gfx.stroke({ color: 0xffffff, width: 4 });

                    sceneManager.entityLayer.addChild(gfx);
                    entityGfxMap.set(sprite.render_id, gfx);
                }

                gfx.position.set(sprite.x, sprite.y);
            }

            // Cleanup despawned / unrendered entities
            for (const [id, gfx] of entityGfxMap.entries()) {
                if (!currentRenderIds.has(id)) {
                    sceneManager.entityLayer.removeChild(gfx);
                    gfx.destroy();
                    entityGfxMap.delete(id);
                }
            }
        }
    }

        // Update HUD Diagnostics
        statusText.text = `Backend: WebGPU/WebGL2 | WASM: ${wasmBridge.isLoaded() ? 'Loaded' : 'Standalone'} | Server: ${wsUrl}\n` +
            `Diagnostics: ${wasmBridge.getDiagnostics()}\n` +
            `Controls: WASD to Move Player (Server Authoritative) | Mouse to Pan/Zoom`;

        cameraTransform.updateScreenSize(app.screen.width, app.screen.height);
        cameraController.update(delta);
        inputAdapter.endFrame();
    });

    loop.start();
}

bootstrap().catch(console.error);
