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
    const canvas = document.createElement('canvas');
    document.body.appendChild(canvas);
    canvas.style.width = '100%';
    canvas.style.height = '100%';
    canvas.style.display = 'block';

    // 1. Init PixiJS
    const pixiApp = new PixiApplication();
    await pixiApp.init(canvas);
    const app = pixiApp.getApp();

    const resizeManager = new ResizeManager(app);
    resizeManager.init();

    const deviceRecovery = new DeviceRecovery(app);
    deviceRecovery.init();

    // 2. Scene
    const sceneManager = new SceneManager(app);

    // 3. Camera
    const cameraTransform = new CameraTransform(sceneManager.worldRoot);
    const cameraController = new CameraController(cameraTransform);

    // 4. Input
    const inputAdapter = new BrowserInputAdapter();
    inputAdapter.attach(canvas);

    // 5. WASM
    const wasmBridge = new WasmBridge();
    // await wasmBridge.load('path/to/module.wasm');

    // 6. Transport
    const transportBridge = new TransportBridge();
    // await transportBridge.connect('ws://localhost:8080');

    // 7. Game Loop
    const loop = new RenderLoop();
    loop.addCallback((delta: number) => {
        cameraTransform.updateScreenSize(app.screen.width, app.screen.height);
        cameraController.update(delta);
        inputAdapter.endFrame();
    });
    
    loop.start();
}

bootstrap().catch(console.error);
