import { Application } from 'pixi.js';

export default class ResizeManager {
    private app: Application;

    constructor(app: Application) {
        this.app = app;
    }

    public init(): void {
        window.addEventListener('resize', this.onResize.bind(this));
        this.onResize();
    }

    public destroy(): void {
        window.removeEventListener('resize', this.onResize.bind(this));
    }

    private onResize(): void {
        this.app.renderer.resize(window.innerWidth, window.innerHeight);
    }

    public toggleFullscreen(): void {
        if (!document.fullscreenElement) {
            document.documentElement.requestFullscreen().catch(err => {
                console.error(`Error attempting to enable fullscreen: ${err.message} (${err.name})`);
            });
        } else {
            if (document.exitFullscreen) {
                document.exitFullscreen();
            }
        }
    }
}
