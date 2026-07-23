export class RenderFrameDecoder {
    public decode(buffer: ArrayBuffer | SharedArrayBuffer): any {
        // Implement binary decoding of RenderFrame from TypedArray
        // For now, return stub data
        return {
            entities: [],
            lights: [],
            camera: { x: 0, y: 0, zoom: 1 }
        };
    }
}
