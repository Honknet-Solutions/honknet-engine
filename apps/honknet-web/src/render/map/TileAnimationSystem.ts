import { Sprite, Texture } from 'pixi.js';

export interface AnimatedTile {
    sprite: Sprite;
    frames: string[];
    currentFrame: number;
    frameTime: number;
    timeAccumulator: number;
}

export class TileAnimationSystem {
    private animatedTiles: AnimatedTile[] = [];

    public register(sprite: Sprite, frames: string[], fps: number): void {
        this.animatedTiles.push({
            sprite,
            frames,
            currentFrame: 0,
            frameTime: 1.0 / fps,
            timeAccumulator: 0
        });
    }

    public update(deltaSeconds: number, textures: Record<string, Texture>): void {
        for (const tile of this.animatedTiles) {
            tile.timeAccumulator += deltaSeconds;
            if (tile.timeAccumulator >= tile.frameTime) {
                tile.timeAccumulator -= tile.frameTime;
                tile.currentFrame = (tile.currentFrame + 1) % tile.frames.length;
                tile.sprite.texture = textures[tile.frames[tile.currentFrame]];
            }
        }
    }
}
