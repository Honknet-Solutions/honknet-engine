import { Container } from 'pixi.js';

export default class EntityRenderRegistry {
    private entities: Map<number, Container> = new Map();

    public register(entityId: number, container: Container): void {
        this.entities.set(entityId, container);
    }

    public unregister(entityId: number): void {
        this.entities.delete(entityId);
    }

    public get(entityId: number): Container | undefined {
        return this.entities.get(entityId);
    }

    public has(entityId: number): boolean {
        return this.entities.has(entityId);
    }

    public getAll(): Map<number, Container> {
        return this.entities;
    }
}
