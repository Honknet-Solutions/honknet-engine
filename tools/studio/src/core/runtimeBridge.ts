import type { RuntimeEntity } from './types';

export type RuntimeMessage = {
  timestamp: number;
  direction: 'in' | 'out' | 'system';
  type: string;
  payload: unknown;
  bytes: number;
};

export type RuntimeState = {
  connected: boolean;
  url: string;
  tick: number;
  playerEntityId: number | null;
  entities: RuntimeEntity[];
  messages: RuntimeMessage[];
  error: string | null;
};

type Listener = (state: RuntimeState) => void;

export class RuntimeBridge {
  private socket: WebSocket | null = null;
  private hotReloadSocket: WebSocket | null = null;
  private hotReloadReconnect: number | null = null;
  private hotReloadDisposed = false;
  private readonly hotReloadQueue: string[] = [];
  private readonly entities = new Map<number, RuntimeEntity>();
  private readonly listeners = new Set<Listener>();
  private readonly hotReloadChannel = typeof BroadcastChannel !== 'undefined'
    ? new BroadcastChannel('honknet-studio-hot-reload')
    : null;
  private state: RuntimeState = {
    connected: false,
    url: 'ws://127.0.0.1:3015',
    tick: 0,
    playerEntityId: null,
    entities: [],
    messages: [],
    error: null,
  };

  public constructor() {
    this.connectHotReloadRelay();
  }

  public get snapshot(): RuntimeState {
    return this.state;
  }

  public subscribe(listener: Listener): () => void {
    this.listeners.add(listener);
    listener(this.state);
    return () => this.listeners.delete(listener);
  }

  public connect(url: string): void {
    this.disconnect();
    this.state = { ...this.state, url, error: null, messages: [] };
    this.emit();
    const socket = new WebSocket(url);
    this.socket = socket;
    socket.addEventListener('open', () => {
      this.state = { ...this.state, connected: true, error: null };
      this.record('system', 'Connected', { url });
      this.send({
        type: 'Hello',
        data: {
          protocol_version: 1,
          client_version: 'honknet-studio-3.0.0',
          identity_id: `studio-inspector-${crypto.randomUUID()}`,
        },
      });
    });
    socket.addEventListener('message', (event) => {
      const text = typeof event.data === 'string' ? event.data : '';
      try {
        const message = JSON.parse(text) as unknown;
        this.record('in', messageType(message), message, text.length);
        this.applyMessage(message);
      } catch (error) {
        this.record('in', 'Malformed', text, text.length);
        this.state = { ...this.state, error: error instanceof Error ? error.message : String(error) };
        this.emit();
      }
    });
    socket.addEventListener('error', () => {
      this.state = { ...this.state, error: `WebSocket connection failed: ${url}` };
      this.emit();
    });
    socket.addEventListener('close', (event) => {
      this.socket = null;
      this.state = { ...this.state, connected: false };
      this.record('system', 'Disconnected', { code: event.code, reason: event.reason });
    });
  }

  public disconnect(): void {
    this.socket?.close(1000, 'Studio disconnect');
    this.socket = null;
    this.entities.clear();
    this.state = { ...this.state, connected: false, playerEntityId: null, entities: [], tick: 0 };
    this.emit();
  }

  public send(payload: unknown): void {
    if (!this.socket || this.socket.readyState !== WebSocket.OPEN) return;
    const text = JSON.stringify(payload);
    this.socket.send(text);
    this.record('out', messageType(payload), payload, text.length);
  }

  public publishHotReload(path: string, content: string): void {
    const message = { type: 'HotReload', path, content, timestamp: Date.now() };
    this.hotReloadChannel?.postMessage(message);
    const encoded = JSON.stringify(message);
    if (this.hotReloadSocket?.readyState === WebSocket.OPEN) this.hotReloadSocket.send(encoded);
    else {
      this.hotReloadQueue.push(encoded);
      if (this.hotReloadQueue.length > 50) this.hotReloadQueue.shift();
      this.connectHotReloadRelay();
    }
    this.record('system', 'HotReload', { path, bytes: content.length });
  }

  public clearMessages(): void {
    this.state = { ...this.state, messages: [] };
    this.emit();
  }

  public dispose(): void {
    this.disconnect();
    this.hotReloadDisposed = true;
    if (this.hotReloadReconnect !== null) window.clearTimeout(this.hotReloadReconnect);
    this.hotReloadReconnect = null;
    this.hotReloadSocket?.close();
    this.hotReloadSocket = null;
    this.hotReloadChannel?.close();
    this.listeners.clear();
  }

  private connectHotReloadRelay(): void {
    if (this.hotReloadDisposed || typeof WebSocket === 'undefined') return;
    if (this.hotReloadSocket && (this.hotReloadSocket.readyState === WebSocket.OPEN || this.hotReloadSocket.readyState === WebSocket.CONNECTING)) return;
    const url = localStorage.getItem('honknet.hotReloadUrl')?.trim() || 'ws://127.0.0.1:3016';
    try {
      const socket = new WebSocket(url);
      this.hotReloadSocket = socket;
      socket.addEventListener('open', () => {
        while (this.hotReloadQueue.length > 0 && socket.readyState === WebSocket.OPEN) {
          const payload = this.hotReloadQueue.shift();
          if (payload) socket.send(payload);
        }
      });
      socket.addEventListener('close', () => {
        if (this.hotReloadSocket === socket) this.hotReloadSocket = null;
        this.scheduleHotReloadReconnect();
      });
      socket.addEventListener('error', () => socket.close());
    } catch {
      this.scheduleHotReloadReconnect();
    }
  }

  private scheduleHotReloadReconnect(): void {
    if (this.hotReloadDisposed || this.hotReloadReconnect !== null) return;
    this.hotReloadReconnect = window.setTimeout(() => {
      this.hotReloadReconnect = null;
      this.connectHotReloadRelay();
    }, 2000);
  }

  private applyMessage(value: unknown): void {
    if (!isRecord(value) || typeof value.type !== 'string') return;
    const data = isRecord(value.data) ? value.data : {};
    switch (value.type) {
      case 'Welcome': {
        this.state = {
          ...this.state,
          playerEntityId: typeof data.entity_net_id === 'number' ? data.entity_net_id : null,
        };
        break;
      }
      case 'Snapshot': {
        this.entities.clear();
        for (const entity of readEntities(data.entities)) this.entities.set(entity.netId, entity);
        this.updateEntityState(typeof data.tick === 'number' ? data.tick : this.state.tick);
        break;
      }
      case 'StateDelta': {
        for (const entity of readEntities(data.spawns)) this.entities.set(entity.netId, entity);
        for (const entity of readEntities(data.updates)) this.entities.set(entity.netId, entity);
        if (Array.isArray(data.despawns)) {
          for (const id of data.despawns) if (typeof id === 'number') this.entities.delete(id);
        }
        this.updateEntityState(typeof data.tick === 'number' ? data.tick : this.state.tick);
        break;
      }
      case 'Error': {
        this.state = { ...this.state, error: typeof data.message === 'string' ? data.message : 'Server error' };
        break;
      }
      default:
        break;
    }
    this.emit();
  }

  private updateEntityState(tick: number): void {
    this.state = {
      ...this.state,
      tick,
      entities: [...this.entities.values()].sort((left, right) => left.netId - right.netId),
    };
  }

  private record(direction: RuntimeMessage['direction'], type: string, payload: unknown, bytes?: number): void {
    const messages = [...this.state.messages, {
      timestamp: Date.now(),
      direction,
      type,
      payload,
      bytes: bytes ?? JSON.stringify(payload).length,
    }].slice(-500);
    this.state = { ...this.state, messages };
    this.emit();
  }

  private emit(): void {
    for (const listener of this.listeners) listener(this.state);
  }
}

function readEntities(value: unknown): RuntimeEntity[] {
  if (!Array.isArray(value)) return [];
  return value.filter(isRecord).flatMap((entity) => {
    const position = isRecord(entity.position) ? entity.position : null;
    if (typeof entity.net_id !== 'number' || typeof entity.prototype !== 'string' || !position) return [];
    return [{
      netId: entity.net_id,
      prototype: entity.prototype,
      position: {
        x: Number(position.x ?? 0),
        y: Number(position.y ?? 0),
        z: Number(position.z ?? 0),
      },
      components: Array.isArray(entity.components) ? entity.components : [],
    }];
  });
}

function messageType(value: unknown): string {
  return isRecord(value) && typeof value.type === 'string' ? value.type : 'Unknown';
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}
