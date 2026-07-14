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
const IGNORE_DISTANCE = 0.002;
const CORRECTION_SPEED = 18;
const MAX_HISTORY = 512;

export type LocalPlayerControllerState = {
  clientSimulationTick: number;
  lastProcessedInputSeq: number | null;
  lastProcessedClientTick: number | null;
  predictedPlayerPosition: NetPosition | null;
  pendingInputCount: number;
  predictionError: number;
};

type Options = {
  getMovement: () => Vec2;
  isConnected: () => boolean;
  sendMessage: (message: ClientMessage) => boolean;
  onFrame: (state: LocalPlayerControllerState) => void;
  onPredictionSnap?: (distance: number) => void;
};

type InputSample = {
  tick: number;
  movement: Vec2;
};

export class LocalPlayerController {
  private playerEntityNetId: EntityNetId | null = null;
  private predicted: NetPosition | null = null;
  private rendered: NetPosition | null = null;
  private clientTick = 0;
  private sequence = 0;
  private lastSentMovement: Vec2 | null = null;
  private lastSentTick: number | null = null;
  private lastAckSeq: number | null = null;
  private lastAckTick: number | null = null;
  private pendingSequences: number[] = [];
  private history: InputSample[] = [];
  private accumulator = 0;
  private lastFrame = performance.now();
  private frameId: number | null = null;
  private predictionError = 0;

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

    if (ackTick !== null) {
      this.history = this.history.filter(
        (sample) => isSequenceNewer(sample.tick, ackTick),
      );
    }

    if (!entity) {
      this.emit();
      return;
    }

    const replayed = replay(entity.position, this.history);

    if (!this.predicted || !this.rendered) {
      this.predicted = { ...replayed };
      this.rendered = { ...replayed };
      this.predictionError = 0;
      this.emit();
      return;
    }

    if (this.predicted.z !== replayed.z) {
      this.predicted = { ...replayed };
      this.rendered = { ...replayed };
      this.predictionError = 0;
      this.emit();
      return;
    }

    const error = Math.hypot(
      replayed.x - this.predicted.x,
      replayed.y - this.predicted.y,
    );
    this.predictionError = error;
    this.predicted = replayed;

    if (error >= SNAP_DISTANCE) {
      this.rendered = { ...replayed };
      this.options.onPredictionSnap?.(error);
    }

    this.emit();
  }

  public getState(): LocalPlayerControllerState {
    return {
      clientSimulationTick: this.clientTick,
      lastProcessedInputSeq: this.lastAckSeq,
      lastProcessedClientTick: this.lastAckTick,
      predictedPlayerPosition: this.rendered ? { ...this.rendered } : null,
      pendingInputCount: this.pendingSequences.length,
      predictionError: this.predictionError,
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

    if (this.predicted && this.rendered) {
      const errorX = this.predicted.x - this.rendered.x;
      const errorY = this.predicted.y - this.rendered.y;
      const distance = Math.hypot(errorX, errorY);
      if (distance > IGNORE_DISTANCE) {
        const factor = 1 - Math.exp(-CORRECTION_SPEED * delta);
        this.rendered.x += errorX * factor;
        this.rendered.y += errorY * factor;
      } else {
        this.rendered.x = this.predicted.x;
        this.rendered.y = this.predicted.y;
      }
      this.rendered.z = this.predicted.z;
    }

    this.emit();
    this.frameId = requestAnimationFrame(this.frame);
  };

  private tick(): void {
    this.clientTick = (this.clientTick + 1) >>> 0;
    const movement = normalize(this.options.getMovement());

    this.history.push({
      tick: this.clientTick,
      movement: { ...movement },
    });
    if (this.history.length > MAX_HISTORY) {
      this.history.splice(0, this.history.length - MAX_HISTORY);
    }

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

    if (this.predicted && this.rendered) {
      const dx = movement.x * MOVE_SPEED * TICK_DELTA;
      const dy = movement.y * MOVE_SPEED * TICK_DELTA;
      this.predicted.x += dx;
      this.predicted.y += dy;
      this.rendered.x += dx;
      this.rendered.y += dy;
    }
  }

  private reset(): void {
    this.predicted = null;
    this.rendered = null;
    this.clientTick = 0;
    this.sequence = 0;
    this.lastSentMovement = null;
    this.lastSentTick = null;
    this.lastAckSeq = null;
    this.lastAckTick = null;
    this.pendingSequences = [];
    this.history = [];
    this.accumulator = 0;
    this.predictionError = 0;
    this.emit();
  }

  private emit(): void {
    this.options.onFrame(this.getState());
  }
}

function replay(authoritative: NetPosition, samples: readonly InputSample[]): NetPosition {
  const result = { ...authoritative };
  for (const sample of samples) {
    result.x += sample.movement.x * MOVE_SPEED * TICK_DELTA;
    result.y += sample.movement.y * MOVE_SPEED * TICK_DELTA;
  }
  return result;
}

function normalize(movement: Vec2): Vec2 {
  const length = Math.hypot(movement.x, movement.y);
  if (length <= 1 || length === 0) return movement;
  return { x: movement.x / length, y: movement.y / length };
}

function isSequenceNewer(candidate: number, current: number): boolean {
  const difference = (candidate - current) >>> 0;
  return difference !== 0 && difference < 0x80000000;
}
