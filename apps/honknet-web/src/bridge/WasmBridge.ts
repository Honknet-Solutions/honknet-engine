export interface RenderSprite {
    render_id: number;
    entity_id: number | bigint;
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

export interface GameActionResult {
    sequence: number;
    status: string;
    server_tick: number | bigint;
}

export interface OwnerHudState {
    mob_state?: string;
    hands?: { active_hand: number; held_item?: unknown; maximum_hands: number };
    equipment?: { slots: Array<[string, unknown]> };
    medical?: {
        blood_fraction: number;
        oxygen_saturation: number;
        pain: number;
        shock: number;
        conscious: boolean;
    };
    interaction?: {
        grab_strength?: string;
        action_kind?: string;
        action_started_tick?: number;
        action_completes_tick?: number;
    };
}

export interface LobbyState {
    phase: string;
    round_id: number | bigint;
    ready_players: number;
    connected_players: number;
    countdown_ticks_remaining: number | bigint;
    assigned_job?: string;
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

    public createLobbyReadyPayload(ready: boolean, preferredJob: string): Uint8Array | null {
        if (!this.runtime || typeof this.runtime.create_lobby_ready_payload !== 'function') return null;
        return this.runtime.create_lobby_ready_payload(ready, preferredJob);
    }

    public getLobbyState(): LobbyState | null {
        if (!this.runtime || typeof this.runtime.get_lobby_state !== 'function') return null;
        return this.runtime.get_lobby_state() as LobbyState | null;
    }

    public createActionPayload(
        sequence: number,
        action:
            | 'interact' | 'attack' | 'pickup'
            | 'bandage' | 'bruise' | 'burn' | 'cpr'
            | 'surgeryChest'
            | 'grab' | 'releaseGrab' | 'pull' | 'stopPulling'
            | 'carry' | 'dropCarried'
            | 'buckle' | 'unbuckle'
            | 'equipJumpsuit' | 'unequipJumpsuit' | 'store' | 'drop',
        entityId: number | bigint = 0,
    ): Uint8Array | null {
        if (!this.runtime || typeof this.runtime.create_action_payload !== 'function') return null;
        return this.runtime.create_action_payload(sequence, action, BigInt(entityId));
    }

    public drainActionResults(): GameActionResult[] {
        if (!this.runtime || typeof this.runtime.drain_action_results !== 'function') return [];
        return this.runtime.drain_action_results() as GameActionResult[];
    }

    public getHudState(): OwnerHudState {
        if (!this.runtime || typeof this.runtime.get_hud_state !== 'function') return {};
        return this.runtime.get_hud_state() as OwnerHudState;
    }

    public getDiagnostics(): string {
        return this.runtime ? this.runtime.get_diagnostics() : 'WASM Not Initialized';
    }
}
