import { Container, Sprite, Texture } from 'pixi.js';
import { TileChunkCache } from './TileChunkCache';

export interface ChunkData {
    x: number;
    y: number;
    tiles: { x: number, y: number, textureId: string }[];
}

export class TileChunkRenderer {
    public layer: Container;
    private cache: TileChunkCache;
    private tileSize: number;

    constructor(layer: Container, tileSize: number = 32) {
        this.layer = layer;
        this.cache = new TileChunkCache();
        this.tileSize = tileSize;
    }

    public renderChunk(data: ChunkData, textures: Record<string, Texture>): void {
        let chunk = this.cache.getChunk(data.x, data.y);
        
        if (chunk) {
            this.layer.removeChild(chunk);
            this.cache.removeChunk(data.x, data.y);
        }

        chunk = new Container();
        chunk.position.set(data.x * this.tileSize * 16, data.y * this.tileSize * 16); // Assuming 16x16 chunk

        for (const tile of data.tiles) {
            const sprite = new Sprite(textures[tile.textureId]);
            sprite.position.set(tile.x * this.tileSize, tile.y * this.tileSize);
            sprite.width = this.tileSize;
            sprite.height = this.tileSize;
            chunk.addChild(sprite);
        }

        this.cache.setChunk(data.x, data.y, chunk);
        this.layer.addChild(chunk);
    }
}
