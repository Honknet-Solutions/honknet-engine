import { Application } from 'pixi.js';
import RendererCapabilities from './RendererCapabilities';

export default class PixiApplication {
    public app: Application;

    constructor() {
        this.app = new Application();
    }

    public async init(canvas: HTMLCanvasElement): Promise<void> {
        const preferWebGPU = await RendererCapabilities.isWebGPUSupported();
        
        await this.app.init({
            canvas: canvas,
            resizeTo: window,
            preference: preferWebGPU ? 'webgpu' : 'webgl',
            backgroundAlpha: 1,
            backgroundColor: 0x000000,
            resolution: window.devicePixelRatio || 1,
            autoDensity: true,
            antialias: false,
        });
    }

    public getApp(): Application {
        return this.app;
    }
}
