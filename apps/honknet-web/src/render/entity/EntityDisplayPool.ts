import { Container, Sprite } from 'pixi.js';

export default class EntityDisplayPool {
    private containerPool: Container[] = [];
    private spritePool: Sprite[] = [];

    public getContainer(): Container {
        if (this.containerPool.length > 0) {
            const container = this.containerPool.pop()!;
            container.visible = true;
            return container;
        }
        return new Container();
    }

    public releaseContainer(container: Container): void {
        container.removeChildren();
        container.visible = false;
        container.position.set(0, 0);
        container.rotation = 0;
        container.scale.set(1, 1);
        this.containerPool.push(container);
    }

    public getSprite(): Sprite {
        if (this.spritePool.length > 0) {
            const sprite = this.spritePool.pop()!;
            sprite.visible = true;
            return sprite;
        }
        return new Sprite();
    }

    public releaseSprite(sprite: Sprite): void {
        sprite.visible = false;
        sprite.position.set(0, 0);
        sprite.rotation = 0;
        sprite.scale.set(1, 1);
        sprite.alpha = 1;
        sprite.tint = 0xFFFFFF;
        // @ts-ignore
        sprite.texture = undefined;
        this.spritePool.push(sprite);
    }
}
