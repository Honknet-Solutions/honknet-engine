export class WasmBridge {
    private wasmInstance: WebAssembly.Instance | null = null;

    public async load(url: string): Promise<void> {
        const response = await fetch(url);
        const buffer = await response.arrayBuffer();
        const module = await WebAssembly.compile(buffer);
        
        this.wasmInstance = await WebAssembly.instantiate(module, {
            env: {
                // environment imports
            }
        });
    }

    public callMethod(method: string, ...args: any[]): any {
        if (!this.wasmInstance) {
            throw new Error('WASM module not loaded');
        }
        const func = this.wasmInstance.exports[method] as Function;
        if (func) {
            return func(...args);
        }
    }
}
