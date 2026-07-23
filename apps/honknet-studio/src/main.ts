import './style.css';
import { ToolMode } from './types';
import { ProjectManagerTool } from './tools/project-manager';
import { MapEditorTool } from './tools/map-editor';
import { UiEditorTool } from './tools/ui-editor';
import { PrototypeEditorTool } from './tools/prototype-editor';
import { ProfilerTool } from './tools/profiler';
import { NetworkInspectorTool } from './tools/network-inspector';
import { MigrationAssistantTool } from './tools/migration-assistant';

class HonknetStudioApp {
    private currentMode: ToolMode = 'project';
    private root = document.querySelector<HTMLElement>('#app')!;
    private logMessages: string[] = ['[STUDIO] Honknet Studio 1.0 initialized. Ready for project workspace.'];

    private projectTool = new ProjectManagerTool();
    private mapTool = new MapEditorTool();
    private uiTool = new UiEditorTool();
    private protoTool = new PrototypeEditorTool();
    private profilerTool = new ProfilerTool();
    private netTool = new NetworkInspectorTool();
    private migrationTool = new MigrationAssistantTool();

    constructor() {
        this.render();
    }

    private log(msg: string): void {
        const timestamp = new Date().toLocaleTimeString();
        this.logMessages.unshift(`[${timestamp}] ${msg}`);
        if (this.logMessages.length > 20) this.logMessages.pop();

        const statusEl = document.getElementById('status-msg');
        if (statusEl) {
            statusEl.innerText = msg;
        }
    }

    private render(): void {
        this.root.innerHTML = `
            <div class="studio-app">
                <header class="studio-header">
                    <div class="brand-section">
                        <span class="logo-icon">🚀</span>
                        <span>HONKNET STUDIO 1.0</span>
                    </div>

                    <nav class="nav-tabs">
                        <button class="tab-btn ${this.currentMode === 'project' ? 'active' : ''}" data-mode="project">📁 Project Manager</button>
                        <button class="tab-btn ${this.currentMode === 'map' ? 'active' : ''}" data-mode="map">🗺️ Map Editor</button>
                        <button class="tab-btn ${this.currentMode === 'ui' ? 'active' : ''}" data-mode="ui">🎨 HUI UI Editor</button>
                        <button class="tab-btn ${this.currentMode === 'prototype' ? 'active' : ''}" data-mode="prototype">🧩 Prototypes & Schemas</button>
                        <button class="tab-btn ${this.currentMode === 'profiler' ? 'active' : ''}" data-mode="profiler">⚡ Profiler</button>
                        <button class="tab-btn ${this.currentMode === 'network' ? 'active' : ''}" data-mode="network">🌐 Network & PVS</button>
                        <button class="tab-btn ${this.currentMode === 'migration' ? 'active' : ''}" data-mode="migration">🔄 Migration Assistant</button>
                    </nav>

                    <div class="action-bar">
                        <button class="btn btn-secondary" id="btn-undo">↩️ Undo</button>
                        <button class="btn btn-secondary" id="btn-redo">↪️ Redo</button>
                        <button class="btn btn-primary" id="btn-play">▶️ Run Preview</button>
                    </div>
                </header>

                <main class="studio-main" id="tool-container">
                    ${this.renderToolPane()}
                </main>

                <footer class="studio-footer">
                    <span id="status-msg">${this.logMessages[0]}</span>
                    <span>Workspace: <code>/workspaces/space-station-15</code> | Honknet Engine 1.0.0-rc.1</span>
                </footer>
            </div>
        `;

        this.bindEvents();
    }

    private renderToolPane(): string {
        switch (this.currentMode) {
            case 'project': return this.projectTool.renderHtml();
            case 'map': return this.mapTool.renderHtml();
            case 'ui': return this.uiTool.renderHtml();
            case 'prototype': return this.protoTool.renderHtml();
            case 'profiler': return this.profilerTool.renderHtml();
            case 'network': return this.netTool.renderHtml();
            case 'migration': return this.migrationTool.renderHtml();
        }
    }

    private bindEvents(): void {
        document.querySelectorAll('.tab-btn').forEach(btn => {
            btn.addEventListener('click', (ev) => {
                const mode = (ev.currentTarget as HTMLElement).dataset.mode as ToolMode;
                if (mode && mode !== this.currentMode) {
                    this.currentMode = mode;
                    this.render();
                    this.log(`Switched view to ${mode.toUpperCase()} tool.`);
                }
            });
        });

        const btnPlay = document.getElementById('btn-play');
        if (btnPlay) {
            btnPlay.onclick = () => this.log('[PLAY] Launching embedded WASM engine preview with live hot-reload...');
        }

        const btnUndo = document.getElementById('btn-undo');
        if (btnUndo) {
            btnUndo.onclick = () => this.log('[UNDO] Reverted last command state.');
        }

        const btnRedo = document.getElementById('btn-redo');
        if (btnRedo) {
            btnRedo.onclick = () => this.log('[REDO] Re-applied command state.');
        }

        // Tool-specific event bindings
        const onLog = (msg: string) => this.log(msg);
        if (this.currentMode === 'project') this.projectTool.bindEvents(onLog);
        if (this.currentMode === 'map') {
            const canvas = document.getElementById('map-canvas') as HTMLCanvasElement;
            if (canvas) this.mapTool.bindCanvas(canvas, onLog);
        }
        if (this.currentMode === 'ui') this.uiTool.bindEvents(onLog);
        if (this.currentMode === 'prototype') this.protoTool.bindEvents(onLog);
        if (this.currentMode === 'migration') this.migrationTool.bindEvents(onLog);
    }
}

new HonknetStudioApp();
