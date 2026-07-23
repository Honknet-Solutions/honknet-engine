export class TransportBridge {
    private ws: WebSocket | null = null;
    private onMessageCallback: ((data: Uint8Array) => void) | null = null;

    public setOnMessage(callback: (data: Uint8Array) => void): void {
        this.onMessageCallback = callback;
    }

    public connect(url: string): Promise<void> {
        return new Promise((resolve, reject) => {
            this.ws = new WebSocket(url);
            this.ws.binaryType = 'arraybuffer';

            this.ws.onopen = () => {
                console.log('[TransportBridge] Connected to server via WebSocket');
                resolve();
            };

            this.ws.onerror = (err) => {
                console.error('[TransportBridge] WebSocket error:', err);
                reject(err);
            };

            this.ws.onmessage = (event) => {
                this.handleMessage(event.data);
            };

            this.ws.onclose = () => {
                console.log('[TransportBridge] Disconnected from server');
            };
        });
    }

    public send(data: ArrayBuffer | Uint8Array): void {
        if (this.ws && this.ws.readyState === WebSocket.OPEN) {
            this.ws.send(data);
        }
    }

    private handleMessage(data: any): void {
        if (data instanceof ArrayBuffer) {
            const bytes = new Uint8Array(data);
            if (this.onMessageCallback) {
                this.onMessageCallback(bytes);
            }
        }
    }
}
