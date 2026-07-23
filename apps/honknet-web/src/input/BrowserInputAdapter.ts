import { KeyboardState } from './KeyboardState';
import { MouseState } from './MouseState';

export class BrowserInputAdapter {
    public keyboard: KeyboardState;
    public mouse: MouseState;

    constructor() {
        this.keyboard = new KeyboardState();
        this.mouse = new MouseState();
    }

    public attach(element: HTMLElement): void {
        window.addEventListener('keydown', (e) => this.keyboard.onKeyDown(e.code));
        window.addEventListener('keyup', (e) => this.keyboard.onKeyUp(e.code));

        element.addEventListener('mousemove', (e) => this.mouse.onMouseMove(e.clientX, e.clientY, e.movementX, e.movementY));
        element.addEventListener('mousedown', (e) => this.mouse.onMouseDown(e.button));
        element.addEventListener('mouseup', (e) => this.mouse.onMouseUp(e.button));
        element.addEventListener('wheel', (e) => this.mouse.onWheel(e.deltaY), { passive: true });
        
        // Prevent context menu
        element.addEventListener('contextmenu', (e) => e.preventDefault());
    }

    public endFrame(): void {
        this.keyboard.endFrame();
        this.mouse.endFrame();
    }
}
