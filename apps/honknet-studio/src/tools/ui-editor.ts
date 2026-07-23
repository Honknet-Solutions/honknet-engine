import { HuiNode } from '../types';

export class UiEditorTool {
    private huiTree: HuiNode = {
        id: 'AirlockControlWindow',
        type: 'Window',
        title: '🔒 Airlock Security Console',
        width: '420px',
        height: '340px',
        backgroundColor: '#0f172a',
        padding: '16px',
        children: [
            {
                id: 'HeaderPanel',
                type: 'Panel',
                flexDirection: 'row',
                backgroundColor: '#1e293b',
                padding: '8px',
                children: [
                    { id: 'StatusLabel', type: 'Label', text: 'Status: LOCKED', color: '#ef4444' },
                    { id: 'PressureBar', type: 'ProgressBar', binding: '$state.pressure', width: '100px' }
                ]
            },
            {
                id: 'ActionButtons',
                type: 'Panel',
                flexDirection: 'column',
                padding: '12px',
                children: [
                    { id: 'CycleBtn', type: 'Button', text: '⚡ Cycle Airlock Chamber', color: '#38bdf8' },
                    { id: 'OverrideBtn', type: 'Button', text: '⚠️ Emergency Manual Override', color: '#f59e0b' },
                    { id: 'DoorList', type: 'VirtualList', binding: '$state.doors', height: '120px' }
                ]
            }
        ]
    };

    private selectedNodeId = 'AirlockControlWindow';

    public renderHtml(): string {
        return `
            <div class="tool-pane ui-pane">
                <div class="ui-editor-layout">
                    <div class="tree-sidebar card">
                        <h3>🌳 HUI Retained Hierarchy</h3>
                        <div class="ui-tree-container">
                            ${this.renderTreeNodes(this.huiTree)}
                        </div>

                        <h3>📦 Widget Palette</h3>
                        <div class="widget-grid">
                            <div class="widget-chip">🔲 Panel</div>
                            <div class="widget-chip">🏷️ Label</div>
                            <div class="widget-chip">🔘 Button</div>
                            <div class="widget-chip">✏️ TextInput</div>
                            <div class="widget-chip">📜 VirtualList</div>
                            <div class="widget-chip">📊 ProgressBar</div>
                            <div class="widget-chip">🖼️ Viewport</div>
                        </div>
                    </div>

                    <div class="preview-stage card">
                        <div class="stage-header">
                            <h3>👁️ Live HUI Document Preview (WASM Engine Rendered)</h3>
                            <span class="badge badge-accent">Interactive HUI Runtime</span>
                        </div>
                        <div class="stage-viewport">
                            <div class="hui-preview-window" style="
                                width: ${this.huiTree.width};
                                height: ${this.huiTree.height};
                                background: ${this.huiTree.backgroundColor};
                                padding: ${this.huiTree.padding};
                            ">
                                <div class="hui-titlebar">${this.huiTree.title}</div>
                                <div class="hui-content">
                                    <div class="hui-panel-row">
                                        <span class="hui-status-red">Status: LOCKED</span>
                                        <div class="hui-progress-bar"><div class="hui-progress-fill" style="width: 75%"></div></div>
                                    </div>
                                    <div class="hui-panel-col">
                                        <button class="hui-btn hui-btn-blue" id="hui-sample-cycle">⚡ Cycle Airlock Chamber</button>
                                        <button class="hui-btn hui-btn-orange" id="hui-sample-override">⚠️ Emergency Manual Override</button>
                                        <div class="hui-list-box">
                                            <div class="hui-list-item">Door #1 - Outer Vacuum (Sealed)</div>
                                            <div class="hui-list-item">Door #2 - Station Corridor (Open)</div>
                                        </div>
                                    </div>
                                </div>
                            </div>
                        </div>
                    </div>

                    <div class="inspector-sidebar card">
                        <h3>⚙️ Node Inspector</h3>
                        <div class="form-grid">
                            <label>
                                <span>Node ID:</span>
                                <input type="text" value="${this.selectedNodeId}" readonly />
                            </label>
                            <label>
                                <span>Widget Type:</span>
                                <input type="text" value="Window" readonly />
                            </label>
                            <label>
                                <span>Title:</span>
                                <input type="text" value="${this.huiTree.title}" id="node-title-input" />
                            </label>
                            <label>
                                <span>Width / Height:</span>
                                <input type="text" value="${this.huiTree.width} x ${this.huiTree.height}" />
                            </label>
                            <label>
                                <span>Data Binding:</span>
                                <input type="text" placeholder="$state.property" value="$state.airlock" />
                            </label>
                            <label>
                                <span>Background:</span>
                                <input type="color" value="#0f172a" />
                            </label>
                        </div>
                    </div>
                </div>
            </div>
        `;
    }

    private renderTreeNodes(node: HuiNode): string {
        const hasChildren = node.children && node.children.length > 0;
        return `
            <div class="tree-node ${node.id === this.selectedNodeId ? 'selected' : ''}" data-node-id="${node.id}">
                <span class="tree-icon">${node.type === 'Window' ? '🪟' : node.type === 'Panel' ? '🔲' : '🔹'}</span>
                <span class="tree-label"><b>${node.id}</b> <small>(${node.type})</small></span>
                ${hasChildren ? `
                    <div class="tree-children">
                        ${node.children!.map(c => this.renderTreeNodes(c)).join('')}
                    </div>
                ` : ''}
            </div>
        `;
    }

    public bindEvents(onLog: (msg: string) => void): void {
        const cycleBtn = document.getElementById('hui-sample-cycle');
        const overrideBtn = document.getElementById('hui-sample-override');

        if (cycleBtn) {
            cycleBtn.onclick = () => onLog('[HUI] Action triggered: Cycle Airlock Chamber (Sending HUI event to server)');
        }
        if (overrideBtn) {
            overrideBtn.onclick = () => onLog('[HUI] Action triggered: Emergency Override (Dispatching HUI event)');
        }
    }
}
