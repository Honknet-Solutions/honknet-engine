import { Container, Graphics, BLEND_MODES } from 'pixi.js';

export interface PointLight {
    x: number;
    y: number;
    radius: number;
    color: number;
    intensity: number;
}

export class LightingManager {
    private lightingLayer: Container;
    private darknessOverlay: Graphics;
    private lights: Graphics[] = [];
    private ambientLight: number = 0x222222;

    constructor(lightingLayer: Container) {
        this.lightingLayer = lightingLayer;
        
        this.darknessOverlay = new Graphics();
        this.darknessOverlay.blendMode = BLEND_MODES.MULTIPLY;
        this.lightingLayer.addChild(this.darknessOverlay);
    }

    public setAmbientLight(color: number): void {
        this.ambientLight = color;
    }

    public update(lights: PointLight[], screenWidth: number, screenHeight: number): void {
        // Clear darkness overlay
        this.darknessOverlay.clear();
        this.darknessOverlay.beginFill(this.ambientLight);
        // Using large rect to cover screen (adjusted via camera scale elsewhere)
        this.darknessOverlay.drawRect(-10000, -10000, 20000, 20000); 
        this.darknessOverlay.endFill();

        // Update light sprites/graphics
        while (this.lights.length < lights.length) {
            const lightGfx = new Graphics();
            lightGfx.blendMode = BLEND_MODES.ADD;
            this.lightingLayer.addChild(lightGfx);
            this.lights.push(lightGfx);
        }

        while (this.lights.length > lights.length) {
            const lightGfx = this.lights.pop()!;
            this.lightingLayer.removeChild(lightGfx);
            lightGfx.destroy();
        }

        for (let i = 0; i < lights.length; i++) {
            const data = lights[i];
            const gfx = this.lights[i];
            
            gfx.clear();
            gfx.beginFill(data.color, data.intensity);
            gfx.drawCircle(0, 0, data.radius);
            gfx.endFill();
            
            gfx.position.set(data.x, data.y);
        }
    }
}
