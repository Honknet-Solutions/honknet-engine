import type {
  ClientMessage,
  ServerMessage,
} from './protocol';

export type ConnectionHandlers = {
  onOpen?: () => void;
  onMessage?: (message: ServerMessage) => void;
  onClose?: () => void;
  onError?: (message: string) => void;
};

export class ClientConnection {
  private socket: WebSocket | null = null;

  public constructor(
    private readonly handlers: ConnectionHandlers = {},
  ) {}

  public get isConnected(): boolean {
    return this.socket?.readyState === WebSocket.OPEN;
  }

  public get isConnecting(): boolean {
    return this.socket?.readyState === WebSocket.CONNECTING;
  }

  public connect(serverUrl: string): boolean {
    if (this.isConnected || this.isConnecting) {
      return false;
    }

    const socket = new WebSocket(serverUrl);
    this.socket = socket;

    socket.addEventListener('open', () => {
      if (this.socket === socket) {
        this.handlers.onOpen?.();
      }
    });

    socket.addEventListener('message', (event: MessageEvent<string>) => {
      if (this.socket !== socket) {
        return;
      }

      try {
        this.handlers.onMessage?.(
          JSON.parse(event.data) as ServerMessage,
        );
      } catch (error) {
        this.handlers.onError?.(
          `Failed to parse server message: ${String(error)}`,
        );
      }
    });

    socket.addEventListener('close', () => {
      if (this.socket === socket) {
        this.socket = null;
        this.handlers.onClose?.();
      }
    });

    socket.addEventListener('error', () => {
      if (this.socket === socket) {
        this.handlers.onError?.(
          'WebSocket error. Is the Rust server running?',
        );
      }
    });

    return true;
  }

  public send(message: ClientMessage): boolean {
    if (!this.socket || this.socket.readyState !== WebSocket.OPEN) {
      return false;
    }

    this.socket.send(JSON.stringify(message));
    return true;
  }

  public disconnect(): void {
    const socket = this.socket;
    this.socket = null;

    if (
      socket &&
      (socket.readyState === WebSocket.OPEN ||
        socket.readyState === WebSocket.CONNECTING)
    ) {
      socket.close();
    }
  }
}
