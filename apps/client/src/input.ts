import type { Vec2 } from './protocol';

type MovementCode =
  | 'KeyW'
  | 'KeyA'
  | 'KeyS'
  | 'KeyD'
  | 'ArrowUp'
  | 'ArrowLeft'
  | 'ArrowDown'
  | 'ArrowRight';

const MOVEMENT_CODES = new Set<string>([
  'KeyW',
  'KeyA',
  'KeyS',
  'KeyD',
  'ArrowUp',
  'ArrowLeft',
  'ArrowDown',
  'ArrowRight',
]);

const KEY_TO_MOVEMENT_CODE = new Map<string, MovementCode>([
  // English
  ['w', 'KeyW'],
  ['a', 'KeyA'],
  ['s', 'KeyS'],
  ['d', 'KeyD'],

  // Russian
  ['ц', 'KeyW'],
  ['ф', 'KeyA'],
  ['ы', 'KeyS'],
  ['в', 'KeyD'],

  // Ukrainian
  ['ц', 'KeyW'],
  ['ф', 'KeyA'],
  ['і', 'KeyS'],
  ['в', 'KeyD'],

  // Arrow keys
  ['arrowup', 'ArrowUp'],
  ['arrowleft', 'ArrowLeft'],
  ['arrowdown', 'ArrowDown'],
  ['arrowright', 'ArrowRight'],
]);

export class InputController {
  private readonly pressedCodes = new Set<MovementCode>();

  public constructor() {
    window.addEventListener('keydown', this.handleKeyDown);
    window.addEventListener('keyup', this.handleKeyUp);
    window.addEventListener('blur', this.clear);

    document.addEventListener(
      'visibilitychange',
      this.handleVisibilityChange,
    );
  }

  public getMovement(): Vec2 {
    let x = 0;
    let y = 0;

    if (
      this.pressedCodes.has('KeyA') ||
      this.pressedCodes.has('ArrowLeft')
    ) {
      x -= 1;
    }

    if (
      this.pressedCodes.has('KeyD') ||
      this.pressedCodes.has('ArrowRight')
    ) {
      x += 1;
    }

    if (
      this.pressedCodes.has('KeyW') ||
      this.pressedCodes.has('ArrowUp')
    ) {
      y -= 1;
    }

    if (
      this.pressedCodes.has('KeyS') ||
      this.pressedCodes.has('ArrowDown')
    ) {
      y += 1;
    }

    return { x, y };
  }

  public destroy(): void {
    window.removeEventListener('keydown', this.handleKeyDown);
    window.removeEventListener('keyup', this.handleKeyUp);
    window.removeEventListener('blur', this.clear);

    document.removeEventListener(
      'visibilitychange',
      this.handleVisibilityChange,
    );

    this.clear();
  }

  private readonly handleKeyDown = (
    event: KeyboardEvent,
  ): void => {
    if (isTextInputTarget(event.target)) {
      return;
    }

    const movementCode = resolveMovementCode(event);

    if (!movementCode) {
      return;
    }

    event.preventDefault();
    this.pressedCodes.add(movementCode);
  };

  private readonly handleKeyUp = (
    event: KeyboardEvent,
  ): void => {
    const movementCode = resolveMovementCode(event);

    if (!movementCode) {
      return;
    }

    event.preventDefault();
    this.pressedCodes.delete(movementCode);
  };

  private readonly handleVisibilityChange = (): void => {
    if (document.hidden) {
      this.clear();
    }
  };

  private readonly clear = (): void => {
    this.pressedCodes.clear();
  };
}

function resolveMovementCode(
  event: KeyboardEvent,
): MovementCode | null {
  if (MOVEMENT_CODES.has(event.code)) {
    return event.code as MovementCode;
  }

  const normalizedKey = event.key.toLowerCase();

  return KEY_TO_MOVEMENT_CODE.get(normalizedKey) ?? null;
}

function isTextInputTarget(target: EventTarget | null): boolean {
  if (!(target instanceof HTMLElement)) {
    return false;
  }

  return (
    target instanceof HTMLInputElement ||
    target instanceof HTMLTextAreaElement ||
    target.isContentEditable
  );
}