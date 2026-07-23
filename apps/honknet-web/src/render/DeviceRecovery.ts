import { Application } from 'pixi.js';

export default class DeviceRecovery {
    private app: Application;
    private onContextLostCallback?: () => void;
    private onContextRestoredCallback?: () => void;

    constructor(app: Application) {
        this.app = app;
    }

    public init(onContextLost?: () => void, onContextRestored?: () => void): void {
        this.onContextLostCallback = onContextLost;
        this.onContextRestoredCallback = onContextRestored;

        const canvas = this.app.canvas;
        canvas.addEventListener('webglcontextlost', this.onContextLost.bind(this), false);
        canvas.addEventListener('webglcontextrestored', this.onContextRestored.bind(this), false);
    }

    public destroy(): void {
        const canvas = this.app.canvas;
        canvas.removeEventListener('webglcontextlost', this.onContextLost.bind(this));
        canvas.removeEventListener('webglcontextrestored', this.onContextRestored.bind(this));
    }

    private onContextLost(event: Event): void {
        event.preventDefault();
        console.warn('WebGL context lost. Attempting recovery...');
        if (this.onContextLostCallback) {
            this.onContextLostCallback();
        }
    }

    private onContextRestored(): void {
        console.info('WebGL context restored.');
        if (this.onContextRestoredCallback) {
            this.onContextRestoredCallback();
        }
    }
}
