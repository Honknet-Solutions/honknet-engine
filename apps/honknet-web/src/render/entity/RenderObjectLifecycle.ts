import EntityRenderRegistry from './EntityRenderRegistry';
import EntityDisplayPool from './EntityDisplayPool';
import SpriteLayerDisplay, { SpriteLayerData } from './SpriteLayerDisplay';
import { Container, Texture } from 'pixi.js';

export interface RenderFrameEntity {
    id: number;
    x: number;
    y: number;
    rotation: number;
    layers: SpriteLayerData[];
}

export default class RenderObjectLifecycle {
    private registry: EntityRenderRegistry;
    private pool: EntityDisplayPool;
    private entityLayer: Container;
    private displays: Map<number, SpriteLayerDisplay> = new Map();

    constructor(registry: EntityRenderRegistry, pool: EntityDisplayPool, entityLayer: Container) {
        this.registry = registry;
        this.pool = pool;
        this.entityLayer = entityLayer;
    }

    public processFrame(entities: RenderFrameEntity[], textures: Record<string, Texture>): void {
        const frameIds = new Set(entities.map(e => e.id));
        const currentIds = new Set(this.registry.getAll().keys());

        // Destroy missing
        for (const id of currentIds) {
            if (!frameIds.has(id)) {
                this.destroyEntity(id);
            }
        }

        // Update or create
        for (const entity of entities) {
            let display = this.displays.get(entity.id);
            if (!display) {
                display = new SpriteLayerDisplay(this.pool);
                this.displays.set(entity.id, display);
                this.registry.register(entity.id, display.root);
                this.entityLayer.addChild(display.root);
            }

            display.root.position.set(entity.x, entity.y);
            display.root.rotation = entity.rotation;
            display.updateLayers(entity.layers, textures);
        }
    }

    private destroyEntity(id: number): void {
        const display = this.displays.get(id);
        if (display) {
            this.entityLayer.removeChild(display.root);
            display.destroy();
            this.displays.delete(id);
        }
        this.registry.unregister(id);
    }
}
