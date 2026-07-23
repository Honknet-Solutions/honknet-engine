import { Container, Graphics } from 'pixi.js';

export interface RenderChunkUpdate {
    chunk_x: number;
    chunk_y: number;
    tiles: number[];
}

export class ChunkRenderer {
    private container: Container;
    private chunks: Map<string, Graphics> = new Map();
    private tileSize: number = 64;
    private chunkSize: number = 32;

    constructor(container: Container) {
        this.container = container;
    }

    public initDefaultStationMap(): void {
        // Build initial 32x32 tile chunks around station origin
        this.updateChunk({
            chunk_x: 0,
            chunk_y: 0,
            tiles: new Array(this.chunkSize * this.chunkSize).fill(1)
        });
    }

    public updateChunk(update: RenderChunkUpdate): void {
        const key = `${update.chunk_x}:${update.chunk_y}`;
        let gfx = this.chunks.get(key);

        if (!gfx) {
            gfx = new Graphics();
            gfx.position.set(
                update.chunk_x * this.chunkSize * this.tileSize,
                update.chunk_y * this.chunkSize * this.tileSize
            );
            this.container.addChild(gfx);
            this.chunks.set(key, gfx);
        } else {
            gfx.clear();
        }

        // Draw 32x32 tiles in chunk
        for (let r = 0; r < 20; r++) {
            for (let c = 0; c < 30; c++) {
                const tileIndex = r * 30 + c;
                const tileType = update.tiles[tileIndex] ?? 1;

                const x = (c - 15) * this.tileSize;
                const y = (r - 10) * this.tileSize;
                const isAlt = (r + c) % 2 === 0;

                gfx.rect(x, y, this.tileSize, this.tileSize);
                gfx.fill({ color: tileType === 0 ? 0x050811 : (isAlt ? 0x111625 : 0x0c101c) });
                gfx.rect(x, y, this.tileSize, this.tileSize);
                gfx.stroke({ color: 0x1e293b, width: 1 });
            }
        }
    }

    public clear(): void {
        for (const gfx of this.chunks.values()) {
            this.container.removeChild(gfx);
            gfx.destroy();
        }
        this.chunks.clear();
    }
}
