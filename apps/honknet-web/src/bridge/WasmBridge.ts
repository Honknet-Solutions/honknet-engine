export interface RenderSprite {
    render_id: number;
    entity_id: number;
    asset_id: number;
    state_id: number;
    frame_id: number;
    direction: number;
    layer: number;
    x: number;
    y: number;
    rotation: number;
    scale_x: number;
    scale_y: number;
    color: number;
    alpha: number;
    flags: number;
}

export interface RenderCamera {
    id: number;
    x: number;
    y: number;
    zoom: number;
}

export interface RenderFrame {
    tick: number;
    interpolation_alpha: number;
    cameras: RenderCamera[];
    sprites: RenderSprite[];
    tiles: any[];
    lights: any[];
    particles: any[];
    ui_commands: any[];
    removals: number[];
}

export class WasmBridge {
    private runtime: any = null;

    public setRuntime(runtime: any): void {
        this.runtime = runtime;
        if (this.runtime && typeof this.runtime.initialize_client === 'function') {
            this.runtime.initialize_client();
        }
    }

    public isLoaded(): boolean {
        return this.runtime !== null;
    }

    public pushNetworkMessage(bytes: Uint8Array): void {
        if (this.runtime) {
            this.runtime.push_network_message(bytes);
        }
    }

    public pushInput(sequence: number, x: number, y: number): void {
        if (this.runtime) {
            this.runtime.push_input(sequence, x, y);
        }
    }

    public tickClient(deltaSeconds: number): void {
        if (this.runtime) {
            this.runtime.tick_client(deltaSeconds);
        }
    }

    public extractRenderFrame(): RenderFrame | null {
        if (!this.runtime) return null;
        return this.runtime.extract_render_frame() as RenderFrame;
    }

    public createInputPayload(sequence: number, x: number, y: number): Uint8Array | null {
        if (!this.runtime) return null;
        return this.runtime.create_input_payload(sequence, x, y);
    }

    public createHelloPayload(): Uint8Array | null {
        if (!this.runtime) return null;
        return this.runtime.create_hello_payload();
    }

    public createAckPayload(ackedTick: number): Uint8Array | null {
        if (!this.runtime || typeof this.runtime.create_ack_payload !== 'function') return null;
        return this.runtime.create_ack_payload(BigInt(ackedTick));
    }

    public getDiagnostics(): string {
        return this.runtime ? this.runtime.get_diagnostics() : 'WASM Not Initialized';
    }
}
