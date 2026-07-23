import { Container } from 'pixi.js';

export class CameraTransform {
    public worldRoot: Container;
    public screenWidth: number = 0;
    public screenHeight: number = 0;

    constructor(worldRoot: Container) {
        this.worldRoot = worldRoot;
    }

    public updateScreenSize(width: number, height: number): void {
        this.screenWidth = width;
        this.screenHeight = height;
    }

    public screenToWorld(screenX: number, screenY: number): { x: number, y: number } {
        const x = (screenX - this.worldRoot.x) / this.worldRoot.scale.x;
        const y = (screenY - this.worldRoot.y) / this.worldRoot.scale.y;
        return { x, y };
    }

    public worldToScreen(worldX: number, worldY: number): { x: number, y: number } {
        const x = worldX * this.worldRoot.scale.x + this.worldRoot.x;
        const y = worldY * this.worldRoot.scale.y + this.worldRoot.y;
        return { x, y };
    }
}
