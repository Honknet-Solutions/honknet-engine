import { Graphics, Text, TextStyle } from 'pixi.js';
import PixiApplication from './render/PixiApplication';
import RenderLoop from './render/RenderLoop';
import ResizeManager from './render/ResizeManager';
import DeviceRecovery from './render/DeviceRecovery';
import SceneManager from './render/scene/SceneManager';
import { CameraController } from './render/camera/CameraController';
import { CameraTransform } from './render/camera/CameraTransform';
import { BrowserInputAdapter } from './input/BrowserInputAdapter';
import { WasmBridge } from './bridge/WasmBridge';
import { TransportBridge } from './bridge/TransportBridge';

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

    // 1. Init PixiJS
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

    // 4. Draw Station Tile Grid (Background & Station Floor)
    const gridGfx = new Graphics();
    const tileSize = 64;
    const gridCols = 30;
    const gridRows = 20;

    for (let r = 0; r < gridRows; r++) {
        for (let c = 0; c < gridCols; c++) {
            const x = (c - gridCols / 2) * tileSize;
            const y = (r - gridRows / 2) * tileSize;

            // Tile fill
            const isAlt = (r + c) % 2 === 0;
            gridGfx.rect(x, y, tileSize, tileSize);
            gridGfx.fill({ color: isAlt ? 0x111625 : 0x0c101c });

            // Grid outline
            gridGfx.rect(x, y, tileSize, tileSize);
            gridGfx.stroke({ color: 0x1e293b, width: 1 });
        }
    }
    sceneManager.tileLayer.addChild(gridGfx);

    // Station Walls
    const wallGfx = new Graphics();
    const minX = (-gridCols / 2) * tileSize;
    const maxX = (gridCols / 2) * tileSize;
    const minY = (-gridRows / 2) * tileSize;
    const maxY = (gridRows / 2) * tileSize;

    wallGfx.rect(minX, minY, maxX - minX, maxY - minY);
    wallGfx.stroke({ color: 0x3b82f6, width: 4 });
    sceneManager.tileLayer.addChild(wallGfx);

    // 5. Interactive Player Entity
    const playerGfx = new Graphics();
    playerGfx.circle(0, 0, 20);
    playerGfx.fill({ color: 0x00f0ff });
    playerGfx.stroke({ color: 0xffffff, width: 3 });

    // Direction indicator
    playerGfx.moveTo(0, 0);
    playerGfx.lineTo(30, 0);
    playerGfx.stroke({ color: 0xffffff, width: 4 });

    sceneManager.entityLayer.addChild(playerGfx);

    let playerX = 0;
    let playerY = 0;

    // 6. HUD UI Elements
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

    const titleText = new Text({ text: 'HONKNET ENGINE v1.0 • WASM + PixiJS 8', style: titleStyle });
    titleText.position.set(20, 20);
    hudContainer.addChild(titleText);

    const statusStyle = new TextStyle({
        fontFamily: 'sans-serif',
        fontSize: 14,
        fill: '#94a3b8',
    });
    const statusText = new Text({
        text: 'Backend: WebGPU/WebGL2 | WASM: Ready | Server: Connected (127.0.0.1:3015)\nControls: WASD to Move Player | Mouse to Pan/Zoom',
        style: statusStyle,
    });
    statusText.position.set(20, 52);
    hudContainer.addChild(statusText);

    // 7. Input
    const inputAdapter = new BrowserInputAdapter();
    inputAdapter.attach(canvas);

    // 8. WASM & Transport Bridges
    const wasmBridge = new WasmBridge();
    const transportBridge = new TransportBridge();
    transportBridge.connect('ws://' + window.location.hostname + ':3015');

    // 9. Game Loop (Layout-Independent Controls: KeyW/KeyA/KeyS/KeyD + Russian ЦФЫВ + Arrows)
    const keysPressed: Record<string, boolean> = {};
    window.addEventListener('keydown', (e) => {
        keysPressed[e.code] = true;
        keysPressed[e.key.toLowerCase()] = true;
    });
    window.addEventListener('keyup', (e) => {
        keysPressed[e.code] = false;
        keysPressed[e.key.toLowerCase()] = false;
    });

    const loop = new RenderLoop();
    loop.addCallback((delta: number) => {
        const speed = 350 * delta;

        const moveUp = keysPressed['KeyW'] || keysPressed['w'] || keysPressed['ц'] || keysPressed['ArrowUp'];
        const moveDown = keysPressed['KeyS'] || keysPressed['s'] || keysPressed['ы'] || keysPressed['ArrowDown'];
        const moveLeft = keysPressed['KeyA'] || keysPressed['a'] || keysPressed['ф'] || keysPressed['ArrowLeft'];
        const moveRight = keysPressed['KeyD'] || keysPressed['d'] || keysPressed['в'] || keysPressed['ArrowRight'];

        if (moveUp) playerY -= speed;
        if (moveDown) playerY += speed;
        if (moveLeft) playerX -= speed;
        if (moveRight) playerX += speed;

        playerGfx.position.set(playerX, playerY);

        cameraTransform.updateScreenSize(app.screen.width, app.screen.height);
        cameraController.update(delta);
        inputAdapter.endFrame();
    });

    loop.start();
}

bootstrap().catch(console.error);
