export class KeyboardState {
    private keys: Set<string> = new Set();
    private justPressed: Set<string> = new Set();
    private justReleased: Set<string> = new Set();

    public onKeyDown(code: string): void {
        if (!this.keys.has(code)) {
            this.justPressed.add(code);
        }
        this.keys.add(code);
    }

    public onKeyUp(code: string): void {
        this.keys.delete(code);
        this.justReleased.add(code);
    }

    public isDown(code: string): boolean {
        return this.keys.has(code);
    }

    public isJustPressed(code: string): boolean {
        return this.justPressed.has(code);
    }

    public isJustReleased(code: string): boolean {
        return this.justReleased.has(code);
    }

    public endFrame(): void {
        this.justPressed.clear();
        this.justReleased.clear();
    }
}
