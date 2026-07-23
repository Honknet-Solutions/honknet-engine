export class MouseState {
    public x: number = 0;
    public y: number = 0;
    public deltaX: number = 0;
    public deltaY: number = 0;
    public wheelDelta: number = 0;

    private buttons: Set<number> = new Set();
    private justPressed: Set<number> = new Set();
    private justReleased: Set<number> = new Set();

    public onMouseMove(x: number, y: number, movementX: number, movementY: number): void {
        this.x = x;
        this.y = y;
        this.deltaX += movementX;
        this.deltaY += movementY;
    }

    public onMouseDown(button: number): void {
        if (!this.buttons.has(button)) {
            this.justPressed.add(button);
        }
        this.buttons.add(button);
    }

    public onMouseUp(button: number): void {
        this.buttons.delete(button);
        this.justReleased.add(button);
    }

    public onWheel(deltaY: number): void {
        this.wheelDelta += deltaY;
    }

    public isDown(button: number): boolean {
        return this.buttons.has(button);
    }

    public isJustPressed(button: number): boolean {
        return this.justPressed.has(button);
    }

    public isJustReleased(button: number): boolean {
        return this.justReleased.has(button);
    }

    public endFrame(): void {
        this.justPressed.clear();
        this.justReleased.clear();
        this.deltaX = 0;
        this.deltaY = 0;
        this.wheelDelta = 0;
    }
}
