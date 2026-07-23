import { Container, Graphics, Texture } from 'pixi.js';

export class TileChunkCache {
    private chunks: Map<string, Container> = new Map();

    public getChunk(x: number, y: number): Container | undefined {
        return this.chunks.get(`${x},${y}`);
    }

    public setChunk(x: number, y: number, container: Container): void {
        this.chunks.set(`${x},${y}`, container);
    }

    public removeChunk(x: number, y: number): void {
        const key = `${x},${y}`;
        const chunk = this.chunks.get(key);
        if (chunk) {
            chunk.destroy({ children: true });
            this.chunks.delete(key);
        }
    }

    public clear(): void {
        for (const chunk of this.chunks.values()) {
            chunk.destroy({ children: true });
        }
        this.chunks.clear();
    }
}
