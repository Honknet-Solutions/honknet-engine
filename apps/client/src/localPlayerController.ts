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
const SNAP_DISTANCE = 0.75;
const IGNORE_DISTANCE = 0.0025;
const CORRECTION_SPEED = 18;
const MAX_HISTORY = 512;
const MAX_FRAME_DELTA = 0.25;
const MAX_SIMULATION_STEPS_PER_FRAME = 8;

type InputSample = {
  sequence: number;
  tick: number;
  movement: Vec2;
};

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
  resolveMovement: (
    position: NetPosition,
    movement: Vec2,
    distance: number,
  ) => NetPosition;
  onFrame: (state: LocalPlayerControllerState) => void;
  onPredictionSnap?: (distance: number) => void;
};

/**
 * Handles local fixed-tick prediction and render-time smoothing.
 *
 * The authoritative/predicted state advances at the simulation tick rate,
 * while the displayed position is extrapolated for the fractional part of
 * the current tick. This keeps movement smooth on 60/120/144 Hz displays
 * without changing the authoritative 30 Hz simulation.
 */
export class LocalPlayerController {
  private playerEntityNetId: EntityNetId | null = null;
  private predicted: NetPosition | null = null;
  private displayed: NetPosition | null = null;
  private visualCorrection: Vec2 = { x: 0, y: 0 };
  private clientTick = 0;
  private sequence = 0;
  private lastAckSeq: number | null = null;
  private lastAckTick: number | null = null;
  private history: InputSample[] = [];
  private accumulator = 0;
  private lastFrame = performance.now();
  private frameId: number | null = null;
  private predictionError = 0;

  public constructor(private readonly options: Options) {}

  public start(): void {
    if (this.frameId !== null) {
      return;
    }

    this.lastFrame = performance.now();
    this.frameId = requestAnimationFrame(this.frame);
  }

  public stop(): void {
    if (this.frameId === null) {
      return;
    }

    cancelAnimationFrame(this.frameId);
    this.frameId = null;
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
      this.history = this.history.filter(
        (sample) => isSequenceNewer(sample.sequence, ackSeq),
      );
    }

    if (!entity) {
      this.emit();
      return;
    }

    const replayed = replay(
      entity.position,
      this.history,
      this.options.resolveMovement,
    );

    if (!this.predicted || !this.displayed) {
      this.predicted = { ...replayed };
      this.displayed = { ...replayed };
      this.visualCorrection = { x: 0, y: 0 };
      this.predictionError = 0;
      this.emit();
      return;
    }

    if (this.predicted.z !== replayed.z) {
      this.predicted = { ...replayed };
      this.displayed = { ...replayed };
      this.visualCorrection = { x: 0, y: 0 };
      this.predictionError = 0;
      this.emit();
      return;
    }

    const previousPredicted = this.predicted;
    const errorX = replayed.x - previousPredicted.x;
    const errorY = replayed.y - previousPredicted.y;
    const errorDistance = Math.hypot(errorX, errorY);

    this.predictionError = errorDistance;
    this.predicted = replayed;

    if (errorDistance >= SNAP_DISTANCE) {
      this.displayed = { ...replayed };
      this.visualCorrection = { x: 0, y: 0 };
      this.options.onPredictionSnap?.(errorDistance);
      this.emit();
      return;
    }

    if (errorDistance > IGNORE_DISTANCE) {
      // Keep the current rendered position continuous. The correction offset
      // decays smoothly instead of moving the player backwards in one frame.
      this.visualCorrection.x += previousPredicted.x - replayed.x;
      this.visualCorrection.y += previousPredicted.y - replayed.y;
    }

    this.emit();
  }

  public getState(): LocalPlayerControllerState {
    return {
      clientSimulationTick: this.clientTick,
      lastProcessedInputSeq: this.lastAckSeq,
      lastProcessedClientTick: this.lastAckTick,
      predictedPlayerPosition:
        this.displayed === null
          ? null
          : { ...this.displayed },
      pendingInputCount: this.history.length,
      predictionError: this.predictionError,
    };
  }

  private readonly frame = (now: number): void => {
    const delta = Math.min(
      Math.max((now - this.lastFrame) / 1000, 0),
      MAX_FRAME_DELTA,
    );

    this.lastFrame = now;
    this.accumulator += delta;

    let steps = 0;

    while (
      this.accumulator >= TICK_DELTA &&
      steps < MAX_SIMULATION_STEPS_PER_FRAME
    ) {
      this.tick();
      this.accumulator -= TICK_DELTA;
      steps += 1;
    }

    if (
      steps >= MAX_SIMULATION_STEPS_PER_FRAME &&
      this.accumulator >= TICK_DELTA
    ) {
      // Drop an excessive backlog after a long browser pause. Catching up
      // hundreds of inputs would cause a large visible jump and network burst.
      this.accumulator = 0;
    }

    this.updateDisplayedPosition(delta);
    this.emit();
    this.frameId = requestAnimationFrame(this.frame);
  };

  private tick(): void {
    this.clientTick = (this.clientTick + 1) >>> 0;

    if (
      !this.options.isConnected() ||
      this.playerEntityNetId === null ||
      !this.predicted
    ) {
      return;
    }

    const movement = normalize(this.options.getMovement());
    const nextSequence = (this.sequence + 1) >>> 0;

    const sent = this.options.sendMessage({
      type: 'Input',
      data: {
        seq: nextSequence,
        client_tick: this.clientTick,
        movement,
      },
    });

    if (!sent) {
      return;
    }

    this.sequence = nextSequence;
    this.history.push({
      sequence: nextSequence,
      tick: this.clientTick,
      movement: { ...movement },
    });

    if (this.history.length > MAX_HISTORY) {
      this.history.splice(
        0,
        this.history.length - MAX_HISTORY,
      );
    }

    this.predicted = this.options.resolveMovement(
      this.predicted,
      movement,
      MOVE_SPEED * TICK_DELTA,
    );
  }

  private updateDisplayedPosition(deltaSeconds: number): void {
    if (!this.predicted) {
      return;
    }

    const movement = normalize(this.options.getMovement());

    // The fixed simulation has consumed whole ticks. Extrapolate only the
    // remaining fraction, so the visible player advances every render frame.
    const extrapolated =
      this.options.isConnected() &&
      this.playerEntityNetId !== null
        ? this.options.resolveMovement(
            this.predicted,
            movement,
            MOVE_SPEED * this.accumulator,
          )
        : this.predicted;

    const decay = Math.exp(-CORRECTION_SPEED * deltaSeconds);
    this.visualCorrection.x *= decay;
    this.visualCorrection.y *= decay;

    if (
      Math.hypot(
        this.visualCorrection.x,
        this.visualCorrection.y,
      ) <= IGNORE_DISTANCE
    ) {
      this.visualCorrection = { x: 0, y: 0 };
    }

    this.displayed = {
      x: extrapolated.x + this.visualCorrection.x,
      y: extrapolated.y + this.visualCorrection.y,
      z: extrapolated.z,
    };
  }

  private reset(): void {
    this.predicted = null;
    this.displayed = null;
    this.visualCorrection = { x: 0, y: 0 };
    this.clientTick = 0;
    this.sequence = 0;
    this.lastAckSeq = null;
    this.lastAckTick = null;
    this.history = [];
    this.accumulator = 0;
    this.predictionError = 0;
    this.lastFrame = performance.now();
    this.emit();
  }

  private emit(): void {
    this.options.onFrame(this.getState());
  }
}

function replay(
  authoritative: NetPosition,
  samples: readonly InputSample[],
  resolveMovement: Options['resolveMovement'],
): NetPosition {
  let result = { ...authoritative };

  for (const sample of samples) {
    result = resolveMovement(
      result,
      sample.movement,
      MOVE_SPEED * TICK_DELTA,
    );
  }

  return result;
}

function normalize(movement: Vec2): Vec2 {
  if (
    !Number.isFinite(movement.x) ||
    !Number.isFinite(movement.y)
  ) {
    return { x: 0, y: 0 };
  }

  const length = Math.hypot(movement.x, movement.y);

  if (length <= 1 || length === 0) {
    return movement;
  }

  return {
    x: movement.x / length,
    y: movement.y / length,
  };
}

function isSequenceNewer(
  candidate: number,
  current: number,
): boolean {
  const difference = (candidate - current) >>> 0;
  return difference !== 0 && difference < 0x80000000;
}
