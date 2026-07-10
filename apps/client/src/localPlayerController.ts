import type {
  ClientMessage,
  EntityNetId,
  EntitySnapshot,
  NetPosition,
  Vec2,
} from './protocol';

const CLIENT_SIMULATION_TICK_RATE = 30;

const CLIENT_SIMULATION_DELTA_SECONDS =
  1 / CLIENT_SIMULATION_TICK_RATE;

const INPUT_HEARTBEAT_INTERVAL_TICKS = 15;

const MAX_FRAME_DELTA_SECONDS = 0.25;

const MAX_SIMULATION_STEPS_PER_FRAME = 8;

const PLAYER_MOVE_SPEED = 4.0;

const RECONCILIATION_SPEED = 12.0;

const RECONCILIATION_IGNORE_DISTANCE =
  0.0025;

const RECONCILIATION_SNAP_DISTANCE = 2.0;

type PendingInput = {
  sequence: number;
  clientTick: number;
  movement: Vec2;
  sentAtMilliseconds: number;
};

export type LocalPlayerControllerState = {
  clientSimulationTick: number;
  lastProcessedInputSeq: number | null;
  lastProcessedClientTick: number | null;
  predictedPlayerPosition:
    NetPosition | null;
  pendingInputCount: number;
};

export type LocalPlayerControllerOptions = {
  getMovement: () => Vec2;

  isConnected: () => boolean;

  sendMessage: (
    message: ClientMessage,
  ) => boolean;

  onFrame: (
    state: LocalPlayerControllerState,
  ) => void;

  onPredictionSnap?: (
    errorDistance: number,
  ) => void;
};

export class LocalPlayerController {
  private readonly getMovement:
    () => Vec2;

  private readonly isConnected:
    () => boolean;

  private readonly sendMessage: (
    message: ClientMessage,
  ) => boolean;

  private readonly onFrame: (
    state: LocalPlayerControllerState,
  ) => void;

  private readonly onPredictionSnap?:
    (errorDistance: number) => void;

  private playerEntityNetId:
    EntityNetId | null = null;

  private predictedPlayerPosition:
    NetPosition | null = null;

  private pendingPositionCorrection:
    Vec2 = {
      x: 0,
      y: 0,
    };

  private clientSimulationTick = 0;

  private simulationAccumulatorSeconds = 0;

  private lastFrameMilliseconds =
    performance.now();

  private simulationFrameRequestId:
    number | null = null;

  private inputSequence = 0;

  private lastSentMovement:
    Vec2 | null = null;

  private lastInputSendClientTick:
    number | null = null;

  private lastProcessedInputSeq:
    number | null = null;

  private lastProcessedClientTick:
    number | null = null;

  private readonly pendingInputs:
    PendingInput[] = [];

  public constructor(
    options: LocalPlayerControllerOptions,
  ) {
    this.getMovement =
      options.getMovement;

    this.isConnected =
      options.isConnected;

    this.sendMessage =
      options.sendMessage;

    this.onFrame =
      options.onFrame;

    this.onPredictionSnap =
      options.onPredictionSnap;
  }

  public start(): void {
    if (
      this.simulationFrameRequestId !==
      null
    ) {
      return;
    }

    this.lastFrameMilliseconds =
      performance.now();

    this.simulationFrameRequestId =
      requestAnimationFrame(
        this.updateSimulationFrame,
      );
  }

  public stop(): void {
    if (
      this.simulationFrameRequestId ===
      null
    ) {
      return;
    }

    cancelAnimationFrame(
      this.simulationFrameRequestId,
    );

    this.simulationFrameRequestId =
      null;
  }

  public setPlayerEntity(
    entityNetId: EntityNetId,
  ): void {
    this.playerEntityNetId =
      entityNetId;

    this.resetSessionState();
    this.emitFrame();
  }

  public clearPlayer(): void {
    this.playerEntityNetId = null;

    this.resetSessionState();
    this.emitFrame();
  }

  public handleSnapshot(
    playerEntity:
      EntitySnapshot | undefined,
    lastProcessedInputSeq:
      number | null,
    lastProcessedClientTick:
      number | null,
  ): void {
    this.lastProcessedInputSeq =
      lastProcessedInputSeq;

    this.lastProcessedClientTick =
      lastProcessedClientTick;

    if (
      lastProcessedInputSeq !== null
    ) {
      this.removeAcknowledgedInputs(
        lastProcessedInputSeq,
      );
    }

    if (playerEntity) {
      this.reconcilePlayerPosition(
        playerEntity,
      );
    }

    this.emitFrame();
  }

  public getState():
    LocalPlayerControllerState {
    return {
      clientSimulationTick:
        this.clientSimulationTick,

      lastProcessedInputSeq:
        this.lastProcessedInputSeq,

      lastProcessedClientTick:
        this.lastProcessedClientTick,

      predictedPlayerPosition:
        this.predictedPlayerPosition
          ? {
              ...this
                .predictedPlayerPosition,
            }
          : null,

      pendingInputCount:
        this.pendingInputs.length,
    };
  }

  private readonly updateSimulationFrame = (
    currentMilliseconds: number,
  ): void => {
    const frameDeltaSeconds =
      Math.min(
        Math.max(
          (
            currentMilliseconds -
            this.lastFrameMilliseconds
          ) / 1000,
          0,
        ),
        MAX_FRAME_DELTA_SECONDS,
      );

    this.lastFrameMilliseconds =
      currentMilliseconds;

    this.simulationAccumulatorSeconds +=
      frameDeltaSeconds;

    let completedSimulationSteps = 0;

    while (
      this.simulationAccumulatorSeconds >=
        CLIENT_SIMULATION_DELTA_SECONDS &&
      completedSimulationSteps <
        MAX_SIMULATION_STEPS_PER_FRAME
    ) {
      this.runSimulationTick();

      this.simulationAccumulatorSeconds -=
        CLIENT_SIMULATION_DELTA_SECONDS;

      completedSimulationSteps += 1;
    }

    if (
      completedSimulationSteps >=
        MAX_SIMULATION_STEPS_PER_FRAME &&
      this.simulationAccumulatorSeconds >=
        CLIENT_SIMULATION_DELTA_SECONDS
    ) {
      this.simulationAccumulatorSeconds = 0;
    }

    this.applyPredictionCorrection(
      frameDeltaSeconds,
    );

    this.emitFrame();

    this.simulationFrameRequestId =
      requestAnimationFrame(
        this.updateSimulationFrame,
      );
  };

  private runSimulationTick(): void {
    this.clientSimulationTick =
      (
        this.clientSimulationTick + 1
      ) >>> 0;

    const movement =
      normalizeMovement(
        this.getMovement(),
      );

    this.sendMovementInputForTick(
      movement,
      this.clientSimulationTick,
    );

    this.applyLocalPrediction(
      movement,
      CLIENT_SIMULATION_DELTA_SECONDS,
    );
  }

  private sendMovementInputForTick(
    movement: Vec2,
    clientTick: number,
  ): void {
    if (
      !this.isConnected() ||
      this.playerEntityNetId === null
    ) {
      return;
    }

    const movementChanged =
      this.lastSentMovement === null ||
      movement.x !==
        this.lastSentMovement.x ||
      movement.y !==
        this.lastSentMovement.y;

    const heartbeatRequired =
      this.lastInputSendClientTick ===
        null ||
      hasReachedTickInterval(
        clientTick,
        this.lastInputSendClientTick,
        INPUT_HEARTBEAT_INTERVAL_TICKS,
      );

    if (
      !movementChanged &&
      !heartbeatRequired
    ) {
      return;
    }

    const nextSequence =
      (
        this.inputSequence + 1
      ) >>> 0;

    const sent =
      this.sendMessage({
        type: 'Input',
        data: {
          seq: nextSequence,
          client_tick: clientTick,
          movement,
        },
      });

    if (!sent) {
      return;
    }

    this.inputSequence =
      nextSequence;

    this.lastSentMovement = {
      x: movement.x,
      y: movement.y,
    };

    this.lastInputSendClientTick =
      clientTick;

    this.pendingInputs.push({
      sequence: nextSequence,
      clientTick,
      movement: {
        x: movement.x,
        y: movement.y,
      },
      sentAtMilliseconds:
        performance.now(),
    });
  }

  private applyLocalPrediction(
    movement: Vec2,
    deltaSeconds: number,
  ): void {
    if (
      !this.isConnected() ||
      this.playerEntityNetId === null ||
      this.predictedPlayerPosition ===
        null
    ) {
      return;
    }

    this.predictedPlayerPosition.x +=
      movement.x *
      PLAYER_MOVE_SPEED *
      deltaSeconds;

    this.predictedPlayerPosition.y +=
      movement.y *
      PLAYER_MOVE_SPEED *
      deltaSeconds;
  }

  private reconcilePlayerPosition(
    playerEntity: EntitySnapshot,
  ): void {
    const serverPosition =
      playerEntity.position;

    if (
      this.predictedPlayerPosition ===
      null
    ) {
      this.predictedPlayerPosition = {
        x: serverPosition.x,
        y: serverPosition.y,
        z: serverPosition.z,
      };

      this.pendingPositionCorrection = {
        x: 0,
        y: 0,
      };

      return;
    }

    if (
      this.predictedPlayerPosition.z !==
      serverPosition.z
    ) {
      this.predictedPlayerPosition = {
        x: serverPosition.x,
        y: serverPosition.y,
        z: serverPosition.z,
      };

      this.pendingPositionCorrection = {
        x: 0,
        y: 0,
      };

      return;
    }

    const errorX =
      serverPosition.x -
      this.predictedPlayerPosition.x;

    const errorY =
      serverPosition.y -
      this.predictedPlayerPosition.y;

    const errorDistance =
      Math.hypot(
        errorX,
        errorY,
      );

    if (
      errorDistance <=
      RECONCILIATION_IGNORE_DISTANCE
    ) {
      this.pendingPositionCorrection = {
        x: 0,
        y: 0,
      };

      return;
    }

    if (
      errorDistance >=
      RECONCILIATION_SNAP_DISTANCE
    ) {
      this.predictedPlayerPosition = {
        x: serverPosition.x,
        y: serverPosition.y,
        z: serverPosition.z,
      };

      this.pendingPositionCorrection = {
        x: 0,
        y: 0,
      };

      this.onPredictionSnap?.(
        errorDistance,
      );

      return;
    }

    this.pendingPositionCorrection = {
      x: errorX,
      y: errorY,
    };
  }

  private applyPredictionCorrection(
    deltaSeconds: number,
  ): void {
    if (
      this.predictedPlayerPosition ===
      null
    ) {
      return;
    }

    const remainingDistance =
      Math.hypot(
        this.pendingPositionCorrection.x,
        this.pendingPositionCorrection.y,
      );

    if (
      remainingDistance <=
      RECONCILIATION_IGNORE_DISTANCE
    ) {
      this.pendingPositionCorrection = {
        x: 0,
        y: 0,
      };

      return;
    }

    const correctionFactor =
      1 -
      Math.exp(
        -RECONCILIATION_SPEED *
          deltaSeconds,
      );

    const correctionX =
      this.pendingPositionCorrection.x *
      correctionFactor;

    const correctionY =
      this.pendingPositionCorrection.y *
      correctionFactor;

    this.predictedPlayerPosition.x +=
      correctionX;

    this.predictedPlayerPosition.y +=
      correctionY;

    this.pendingPositionCorrection.x -=
      correctionX;

    this.pendingPositionCorrection.y -=
      correctionY;
  }

  private removeAcknowledgedInputs(
    acknowledgedSequence: number,
  ): void {
    while (
      this.pendingInputs.length > 0
    ) {
      const pendingInput =
        this.pendingInputs[0];

      if (
        !isSequenceAcknowledged(
          pendingInput.sequence,
          acknowledgedSequence,
        )
      ) {
        break;
      }

      this.pendingInputs.shift();
    }
  }

  private resetSessionState(): void {
    this.inputSequence = 0;

    this.lastSentMovement = null;

    this.lastInputSendClientTick = null;

    this.lastProcessedInputSeq = null;

    this.lastProcessedClientTick = null;

    this.predictedPlayerPosition = null;

    this.pendingPositionCorrection = {
      x: 0,
      y: 0,
    };

    this.pendingInputs.length = 0;

    this.clientSimulationTick = 0;

    this.simulationAccumulatorSeconds = 0;

    this.lastFrameMilliseconds =
      performance.now();
  }

  private emitFrame(): void {
    this.onFrame(
      this.getState(),
    );
  }
}

function normalizeMovement(
  movement: Vec2,
): Vec2 {
  if (
    !Number.isFinite(movement.x) ||
    !Number.isFinite(movement.y)
  ) {
    return {
      x: 0,
      y: 0,
    };
  }

  const lengthSquared =
    movement.x * movement.x +
    movement.y * movement.y;

  if (lengthSquared <= 1) {
    return {
      x: movement.x,
      y: movement.y,
    };
  }

  const length =
    Math.sqrt(lengthSquared);

  return {
    x: movement.x / length,
    y: movement.y / length,
  };
}

function hasReachedTickInterval(
  currentTick: number,
  previousTick: number,
  intervalTicks: number,
): boolean {
  const elapsedTicks =
    (
      currentTick - previousTick
    ) >>> 0;

  return (
    elapsedTicks >= intervalTicks
  );
}

function isSequenceAcknowledged(
  sequence: number,
  acknowledgedSequence: number,
): boolean {
  if (
    sequence ===
    acknowledgedSequence
  ) {
    return true;
  }

  return isSequenceNewer(
    acknowledgedSequence,
    sequence,
  );
}

function isSequenceNewer(
  candidate: number,
  current: number,
): boolean {
  const difference =
    (
      candidate - current
    ) >>> 0;

  return (
    difference !== 0 &&
    difference < 0x80000000
  );
}