import { Application, isWebGPUSupported } from 'pixi.js';

export default class RendererCapabilities {
    static async isWebGPUSupported(): Promise<boolean> {
        return await isWebGPUSupported();
    }

    static isWebGL2Supported(): boolean {
        try {
            const canvas = document.createElement('canvas');
            return !!(window.WebGL2RenderingContext && canvas.getContext('webgl2'));
        } catch (e) {
            return false;
        }
    }
}
