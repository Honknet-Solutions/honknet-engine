import type { Vec2 } from './protocol';

const MOVEMENT_CODES = new Set([
  'KeyW', 'KeyA', 'KeyS', 'KeyD',
  'ArrowUp', 'ArrowLeft', 'ArrowDown', 'ArrowRight',
]);

type InputControllerOptions = {
  onInteract?: () => void;
};

export class InputController {
  private readonly pressed = new Set<string>();

  public constructor(
    private readonly options: InputControllerOptions = {},
  ) {
    window.addEventListener('keydown', this.onKeyDown);
    window.addEventListener('keyup', this.onKeyUp);
    window.addEventListener('blur', this.clear);
    document.addEventListener('visibilitychange', this.onVisibilityChange);
  }

  public getMovement(): Vec2 {
    let x = 0;
    let y = 0;

    if (this.pressed.has('KeyA') || this.pressed.has('ArrowLeft')) x -= 1;
    if (this.pressed.has('KeyD') || this.pressed.has('ArrowRight')) x += 1;
    if (this.pressed.has('KeyW') || this.pressed.has('ArrowUp')) y -= 1;
    if (this.pressed.has('KeyS') || this.pressed.has('ArrowDown')) y += 1;

    const length = Math.hypot(x, y);
    return length > 1 ? { x: x / length, y: y / length } : { x, y };
  }

  public destroy(): void {
    window.removeEventListener('keydown', this.onKeyDown);
    window.removeEventListener('keyup', this.onKeyUp);
    window.removeEventListener('blur', this.clear);
    document.removeEventListener('visibilitychange', this.onVisibilityChange);
    this.clear();
  }

  private readonly onKeyDown = (event: KeyboardEvent): void => {
    if (isTextInput(event.target)) {
      return;
    }

    if (event.code === 'KeyE' && !event.repeat) {
      event.preventDefault();
      this.options.onInteract?.();
      return;
    }

    if (MOVEMENT_CODES.has(event.code)) {
      event.preventDefault();
      this.pressed.add(event.code);
    }
  };

  private readonly onKeyUp = (event: KeyboardEvent): void => {
    if (MOVEMENT_CODES.has(event.code)) {
      event.preventDefault();
      this.pressed.delete(event.code);
    }
  };

  private readonly onVisibilityChange = (): void => {
    if (document.hidden) {
      this.clear();
    }
  };

  private readonly clear = (): void => {
    this.pressed.clear();
  };
}

function isTextInput(target: EventTarget | null): boolean {
  return target instanceof HTMLInputElement ||
    target instanceof HTMLTextAreaElement ||
    (target instanceof HTMLElement && target.isContentEditable);
}
