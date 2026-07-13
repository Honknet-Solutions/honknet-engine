import type { ClientEntity } from './clientEntity';
import type {
  ClientMessage,
  EntityNetId,
  NetPosition,
  Vec2,
} from './protocol';

const TICK_RATE = 30;
const TICK_DELTA = 1 / TICK_RATE;
const MOVE_SPEED = 4;
const HEARTBEAT_TICKS = 15;
const SNAP_DISTANCE = 1.5;
const IGNORE_DISTANCE = 0.003;
const CORRECTION_SPEED = 12;

export type LocalPlayerControllerState = {
  clientSimulationTick: number;
  lastProcessedInputSeq: number | null;
  lastProcessedClientTick: number | null;
  predictedPlayerPosition: NetPosition | null;
  pendingInputCount: number;
};

type Options = {
  getMovement: () => Vec2;
  isConnected: () => boolean;
  sendMessage: (message: ClientMessage) => boolean;
  onFrame: (state: LocalPlayerControllerState) => void;
  onPredictionSnap?: (distance: number) => void;
};

export class LocalPlayerController {
  private playerEntityNetId: EntityNetId | null = null;
  private predicted: NetPosition | null = null;
  private correction: Vec2 = { x: 0, y: 0 };
  private clientTick = 0;
  private sequence = 0;
  private lastSentMovement: Vec2 | null = null;
  private lastSentTick: number | null = null;
  private lastAckSeq: number | null = null;
  private lastAckTick: number | null = null;
  private pendingSequences: number[] = [];
  private accumulator = 0;
  private lastFrame = performance.now();
  private frameId: number | null = null;

  public constructor(private readonly options: Options) {}

  public start(): void {
    if (this.frameId !== null) return;
    this.lastFrame = performance.now();
    this.frameId = requestAnimationFrame(this.frame);
  }

  public stop(): void {
    if (this.frameId !== null) {
      cancelAnimationFrame(this.frameId);
      this.frameId = null;
    }
  }

  public setPlayerEntity(netId: EntityNetId): void {
    this.playerEntityNetId = netId;
    this.reset();
  }

  public clearPlayer(): void {
    this.playerEntityNetId = null;
    this.reset();
  }

  public handleSnapshot(
    entity: ClientEntity | undefined,
    ackSeq: number | null,
    ackTick: number | null,
  ): void {
    this.lastAckSeq = ackSeq;
    this.lastAckTick = ackTick;

    if (ackSeq !== null) {
      this.pendingSequences = this.pendingSequences.filter(
        (sequence) => isSequenceNewer(sequence, ackSeq),
      );
    }

    if (!entity) {
      this.emit();
      return;
    }

    if (!this.predicted) {
      this.predicted = { ...entity.position };
      this.emit();
      return;
    }

    if (this.predicted.z !== entity.position.z) {
      this.predicted = { ...entity.position };
      this.correction = { x: 0, y: 0 };
      this.emit();
      return;
    }

    const errorX = entity.position.x - this.predicted.x;
    const errorY = entity.position.y - this.predicted.y;
    const distance = Math.hypot(errorX, errorY);

    if (distance >= SNAP_DISTANCE) {
      this.predicted = { ...entity.position };
      this.correction = { x: 0, y: 0 };
      this.options.onPredictionSnap?.(distance);
    } else if (distance > IGNORE_DISTANCE) {
      this.correction = { x: errorX, y: errorY };
    }

    this.emit();
  }

  public getState(): LocalPlayerControllerState {
    return {
      clientSimulationTick: this.clientTick,
      lastProcessedInputSeq: this.lastAckSeq,
      lastProcessedClientTick: this.lastAckTick,
      predictedPlayerPosition: this.predicted ? { ...this.predicted } : null,
      pendingInputCount: this.pendingSequences.length,
    };
  }

  private readonly frame = (now: number): void => {
    const delta = Math.min(Math.max((now - this.lastFrame) / 1000, 0), 0.25);
    this.lastFrame = now;
    this.accumulator += delta;

    let steps = 0;
    while (this.accumulator >= TICK_DELTA && steps < 8) {
      this.tick();
      this.accumulator -= TICK_DELTA;
      steps += 1;
    }

    const factor = 1 - Math.exp(-CORRECTION_SPEED * delta);
    if (this.predicted) {
      this.predicted.x += this.correction.x * factor;
      this.predicted.y += this.correction.y * factor;
      this.correction.x *= 1 - factor;
      this.correction.y *= 1 - factor;
    }

    this.emit();
    this.frameId = requestAnimationFrame(this.frame);
  };

  private tick(): void {
    this.clientTick = (this.clientTick + 1) >>> 0;
    const movement = this.options.getMovement();

    if (this.options.isConnected() && this.playerEntityNetId !== null) {
      const changed = !this.lastSentMovement ||
        movement.x !== this.lastSentMovement.x ||
        movement.y !== this.lastSentMovement.y;
      const heartbeat = this.lastSentTick === null ||
        ((this.clientTick - this.lastSentTick) >>> 0) >= HEARTBEAT_TICKS;

      if (changed || heartbeat) {
        const nextSequence = (this.sequence + 1) >>> 0;
        if (this.options.sendMessage({
          type: 'Input',
          data: {
            seq: nextSequence,
            client_tick: this.clientTick,
            movement,
          },
        })) {
          this.sequence = nextSequence;
          this.lastSentMovement = { ...movement };
          this.lastSentTick = this.clientTick;
          this.pendingSequences.push(nextSequence);
        }
      }
    }

    if (this.predicted) {
      this.predicted.x += movement.x * MOVE_SPEED * TICK_DELTA;
      this.predicted.y += movement.y * MOVE_SPEED * TICK_DELTA;
    }
  }

  private reset(): void {
    this.predicted = null;
    this.correction = { x: 0, y: 0 };
    this.clientTick = 0;
    this.sequence = 0;
    this.lastSentMovement = null;
    this.lastSentTick = null;
    this.lastAckSeq = null;
    this.lastAckTick = null;
    this.pendingSequences = [];
    this.accumulator = 0;
    this.emit();
  }

  private emit(): void {
    this.options.onFrame(this.getState());
  }
}

function isSequenceNewer(candidate: number, current: number): boolean {
  const difference = (candidate - current) >>> 0;
  return difference !== 0 && difference < 0x80000000;
}
