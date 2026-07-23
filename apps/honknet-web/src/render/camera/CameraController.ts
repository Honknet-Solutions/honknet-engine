import { Container } from 'pixi.js';
import { CameraTransform } from './CameraTransform';

export class CameraController {
    private transform: CameraTransform;
    private targetX: number = 0;
    private targetY: number = 0;
    private currentX: number = 0;
    private currentY: number = 0;
    private zoomTarget: number = 1;
    private currentZoom: number = 1;
    private smoothSpeed: number = 10;
    
    constructor(transform: CameraTransform) {
        this.transform = transform;
    }

    public setTarget(x: number, y: number): void {
        this.targetX = x;
        this.targetY = y;
    }

    public setZoom(zoom: number): void {
        this.zoomTarget = Math.max(0.1, Math.min(zoom, 10));
    }

    public jumpToTarget(): void {
        this.currentX = this.targetX;
        this.currentY = this.targetY;
        this.currentZoom = this.zoomTarget;
        this.apply();
    }

    public update(delta: number): void {
        this.currentX += (this.targetX - this.currentX) * this.smoothSpeed * delta;
        this.currentY += (this.targetY - this.currentY) * this.smoothSpeed * delta;
        this.currentZoom += (this.zoomTarget - this.currentZoom) * this.smoothSpeed * delta;
        this.apply();
    }

    private apply(): void {
        const root = this.transform.worldRoot;
        root.scale.set(this.currentZoom, this.currentZoom);
        
        // Center on screen
        root.x = this.transform.screenWidth / 2 - this.currentX * this.currentZoom;
        root.y = this.transform.screenHeight / 2 - this.currentY * this.currentZoom;
    }
}
