export default class RenderLoop {
    private isRunning: boolean = false;
    private lastTime: number = 0;
    private animationFrameId: number | null = null;
    private updateCallbacks: Set<(delta: number) => void> = new Set();

    public start(): void {
        if (this.isRunning) return;
        this.isRunning = true;
        this.lastTime = performance.now();
        this.loop(this.lastTime);
    }

    public stop(): void {
        this.isRunning = false;
        if (this.animationFrameId !== null) {
            cancelAnimationFrame(this.animationFrameId);
            this.animationFrameId = null;
        }
    }

    public addCallback(callback: (delta: number) => void): void {
        this.updateCallbacks.add(callback);
    }

    public removeCallback(callback: (delta: number) => void): void {
        this.updateCallbacks.delete(callback);
    }

    private loop(time: number): void {
        if (!this.isRunning) return;

        const delta = (time - this.lastTime) / 1000.0;
        this.lastTime = time;

        for (const callback of this.updateCallbacks) {
            callback(delta);
        }

        this.animationFrameId = requestAnimationFrame((nextTime) => this.loop(nextTime));
    }
}
