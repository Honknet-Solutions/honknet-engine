import { Container, Graphics, Sprite, Texture } from 'pixi.js';
import { RenderSprite } from '../bridge/WasmBridge';

export class RsiSpriteRenderer {
    private textureCache: Map<string, Texture> = new Map();

    /**
     * Create or retrieve a procedural/RSI texture for an entity
     */
    public getOrCreateTexture(colorHex: number): Texture {
        const key = `color_${colorHex.toString(16)}`;
        let tex = this.textureCache.get(key);
        if (!tex) {
            const canvas = document.createElement('canvas');
            canvas.width = 32;
            canvas.height = 32;
            const ctx = canvas.getContext('2d');
            if (ctx) {
                // Human body silhouette / sprite
                ctx.fillStyle = `#${colorHex.toString(16).padStart(6, '0')}`;
                ctx.beginPath();
                ctx.arc(16, 16, 14, 0, Math.PI * 2);
                ctx.fill();
                ctx.lineWidth = 2;
                ctx.strokeStyle = '#ffffff';
                ctx.stroke();

                // Direction pointer dot
                ctx.fillStyle = '#ffffff';
                ctx.beginPath();
                ctx.arc(24, 16, 4, 0, Math.PI * 2);
                ctx.fill();
            }
            tex = Texture.from(canvas);
            this.textureCache.set(key, tex);
        }
        return tex;
    }

    /**
     * Build an entity container with sprite layer & direction indicator
     */
    public createEntityContainer(spriteData: RenderSprite): Container {
        const container = new Container();

        const tex = this.getOrCreateTexture(spriteData.color || 0x00f0ff);
        const sprite = new Sprite(tex);
        sprite.anchor.set(0.5, 0.5);

        container.addChild(sprite);

        // Direction indicator
        const dirIndicator = new Graphics();
        dirIndicator.circle(12, 0, 4);
        dirIndicator.fill({ color: 0xffffff });
        container.addChild(dirIndicator);

        return container;
    }

    /**
     * Update position and rotation of entity container
     */
    public updateEntityContainer(container: Container, spriteData: RenderSprite): void {
        container.position.set(spriteData.x, spriteData.y);

        // Map direction (0: South, 1: East, 2: North, 3: West)
        let rotationAngle = 0;
        switch (spriteData.direction) {
            case 1: rotationAngle = Math.PI / 2; break; // East
            case 2: rotationAngle = Math.PI; break;     // North
            case 3: rotationAngle = -Math.PI / 2; break; // West
            default: rotationAngle = 0; break;          // South
        }
        container.rotation = rotationAngle;
    }
}
