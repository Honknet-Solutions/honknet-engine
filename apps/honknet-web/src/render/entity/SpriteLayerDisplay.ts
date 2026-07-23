import { Container, Sprite, Texture } from 'pixi.js';
import EntityDisplayPool from './EntityDisplayPool';

export interface SpriteLayerData {
    textureId: string;
    zIndex: number;
    tint?: number;
    alpha?: number;
}

export default class SpriteLayerDisplay {
    public root: Container;
    private sprites: Map<string, Sprite> = new Map();
    private pool: EntityDisplayPool;

    constructor(pool: EntityDisplayPool) {
        this.pool = pool;
        this.root = this.pool.getContainer();
        this.root.sortableChildren = true;
    }

    public updateLayers(layers: SpriteLayerData[], textures: Record<string, Texture>): void {
        const currentKeys = new Set(this.sprites.keys());
        const newKeys = new Set(layers.map(l => l.textureId));

        // Remove old
        for (const key of currentKeys) {
            if (!newKeys.has(key)) {
                const sprite = this.sprites.get(key)!;
                this.root.removeChild(sprite);
                this.pool.releaseSprite(sprite);
                this.sprites.delete(key);
            }
        }

        // Add or update
        for (const layer of layers) {
            let sprite = this.sprites.get(layer.textureId);
            if (!sprite) {
                sprite = this.pool.getSprite();
                this.sprites.set(layer.textureId, sprite);
                this.root.addChild(sprite);
            }

            sprite.texture = textures[layer.textureId];
            sprite.zIndex = layer.zIndex;
            sprite.tint = layer.tint ?? 0xFFFFFF;
            sprite.alpha = layer.alpha ?? 1;
        }
    }

    public destroy(): void {
        for (const sprite of this.sprites.values()) {
            this.root.removeChild(sprite);
            this.pool.releaseSprite(sprite);
        }
        this.sprites.clear();
        this.pool.releaseContainer(this.root);
    }
}
