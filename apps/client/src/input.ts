import type { Vec2 } from './protocol';

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

export class InputController {
  private readonly pressedCodes = new Set<string>();

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
    if (!MOVEMENT_CODES.has(event.code)) {
      return;
    }

    event.preventDefault();
    this.pressedCodes.add(event.code);
  };

  private readonly handleKeyUp = (
    event: KeyboardEvent,
  ): void => {
    if (!MOVEMENT_CODES.has(event.code)) {
      return;
    }

    event.preventDefault();
    this.pressedCodes.delete(event.code);
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