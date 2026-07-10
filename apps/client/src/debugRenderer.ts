import type {
  EntityNetId,
  EntitySnapshot,
  NetPosition,
  Vec2,
} from './protocol';

const WORLD_SCALE = 32;

export type DebugRendererState = {
  serverTick: number | null;
  playerEntityNetId: EntityNetId | null;
  movement: Vec2;
  entities: ReadonlyMap<EntityNetId, EntitySnapshot>;
};

export class DebugRenderer {
  private readonly canvas: HTMLCanvasElement;
  private readonly context: CanvasRenderingContext2D;

  private state: DebugRendererState = {
    serverTick: null,
    playerEntityNetId: null,
    movement: { x: 0, y: 0 },
    entities: new Map(),
  };

  private animationFrameId: number | null = null;

  public constructor(canvas: HTMLCanvasElement) {
    const context = canvas.getContext('2d');

    if (!context) {
      throw new Error('Failed to create 2D canvas context');
    }

    this.canvas = canvas;
    this.context = context;
  }

  public start(): void {
    if (this.animationFrameId !== null) {
      return;
    }

    this.animationFrameId = window.requestAnimationFrame(
      this.renderFrame,
    );
  }

  public stop(): void {
    if (this.animationFrameId === null) {
      return;
    }

    window.cancelAnimationFrame(this.animationFrameId);
    this.animationFrameId = null;
  }

  public update(state: DebugRendererState): void {
    this.state = state;
  }

  private readonly renderFrame = (): void => {
    this.drawWorld();

    this.animationFrameId = window.requestAnimationFrame(
      this.renderFrame,
    );
  };

  private drawWorld(): void {
    this.context.clearRect(
      0,
      0,
      this.canvas.width,
      this.canvas.height,
    );

    this.drawGrid();

    for (const entity of this.state.entities.values()) {
      this.drawEntity(entity);
    }

    this.drawHud();
  }

  private drawGrid(): void {
    const centerX = this.canvas.width / 2;
    const centerY = this.canvas.height / 2;

    this.context.lineWidth = 1;
    this.context.strokeStyle =
      'rgba(255, 255, 255, 0.08)';

    for (
      let x = centerX % WORLD_SCALE;
      x < this.canvas.width;
      x += WORLD_SCALE
    ) {
      this.context.beginPath();
      this.context.moveTo(x, 0);
      this.context.lineTo(x, this.canvas.height);
      this.context.stroke();
    }

    for (
      let y = centerY % WORLD_SCALE;
      y < this.canvas.height;
      y += WORLD_SCALE
    ) {
      this.context.beginPath();
      this.context.moveTo(0, y);
      this.context.lineTo(this.canvas.width, y);
      this.context.stroke();
    }

    this.context.strokeStyle =
      'rgba(255, 255, 255, 0.3)';

    this.context.beginPath();
    this.context.moveTo(centerX, 0);
    this.context.lineTo(centerX, this.canvas.height);
    this.context.stroke();

    this.context.beginPath();
    this.context.moveTo(0, centerY);
    this.context.lineTo(this.canvas.width, centerY);
    this.context.stroke();
  }

  private drawEntity(entity: EntitySnapshot): void {
    const screenPosition = this.worldToScreen(
      entity.position,
    );

    const isPlayer =
      entity.net_id === this.state.playerEntityNetId;

    this.context.beginPath();

    this.context.arc(
      screenPosition.x,
      screenPosition.y,
      isPlayer ? 12 : 10,
      0,
      Math.PI * 2,
    );

    this.context.fillStyle = isPlayer
      ? '#7cffc4'
      : '#ffcc66';

    this.context.fill();

    this.context.lineWidth = 2;

    this.context.strokeStyle = isPlayer
      ? '#eafff6'
      : '#fff0c2';

    this.context.stroke();

    this.context.fillStyle = '#ffffff';
    this.context.font = '12px monospace';
    this.context.textAlign = 'center';

    this.context.fillText(
      `${entity.net_id}${isPlayer ? ' YOU' : ''}`,
      screenPosition.x,
      screenPosition.y - 18,
    );
  }

  private drawHud(): void {
    this.context.fillStyle = 'rgba(0, 0, 0, 0.45)';
    this.context.fillRect(12, 12, 250, 72);

    this.context.fillStyle = '#ffffff';
    this.context.font = '13px monospace';
    this.context.textAlign = 'left';

    this.context.fillText(
      `tick: ${this.state.serverTick ?? '-'}`,
      24,
      34,
    );

    this.context.fillText(
      `entity: ${this.state.playerEntityNetId ?? '-'}`,
      24,
      54,
    );

    this.context.fillText(
      `input: x=${this.state.movement.x}, y=${this.state.movement.y}`,
      24,
      74,
    );
  }

  private worldToScreen(position: NetPosition): Vec2 {
    return {
      x:
        this.canvas.width / 2 +
        position.x * WORLD_SCALE,
      y:
        this.canvas.height / 2 +
        position.y * WORLD_SCALE,
    };
  }
}