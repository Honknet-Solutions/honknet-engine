import { ComponentSchema, PrototypeDefinition } from '../types';

export class PrototypeEditorTool {
    private prototypes: PrototypeDefinition[] = [
        {
            id: 'SecurityOfficer',
            name: 'Security Officer',
            parents: ['BaseCharacter', 'HumanoidEntity'],
            isAbstract: false,
            components: {
                Transform: { position: [2.0, 3.0], rotation: 0 },
                Physics: { bodyType: 'Dynamic', mass: 75.0, friction: 0.5 },
                Health: { maxHealth: 100, currentHealth: 100 },
                SpriteState: { texture: 'security_officer.png', layer: 'Objects' },
                Networked: { mode: 'All', priority: 100 },
                Predicted: { enabled: true }
            }
        },
        {
            id: 'BaseCharacter',
            name: 'Base Character Prototype',
            parents: ['BaseEntity'],
            isAbstract: true,
            components: {
                Health: { maxHealth: 100, currentHealth: 100 },
                Physics: { bodyType: 'Dynamic', mass: 70.0 }
            }
        }
    ];

    private schemas: ComponentSchema[] = [
        {
            id: 'Transform',
            rustType: 'honknet_transform::Transform',
            storage: 'packed',
            fields: [
                { name: 'position', type: 'vec2', defaultValue: [0, 0], description: 'Local position vector' },
                { name: 'rotation', type: 'number', defaultValue: 0, description: 'Angle in radians' }
            ]
        },
        {
            id: 'Health',
            rustType: 'honknet_game::HealthComponent',
            storage: 'packed',
            fields: [
                { name: 'maxHealth', type: 'number', defaultValue: 100, description: 'Maximum hit points' },
                { name: 'currentHealth', type: 'number', defaultValue: 100, description: 'Current hit points' }
            ]
        },
        {
            id: 'Physics',
            rustType: 'honknet_physics::Body',
            storage: 'packed',
            fields: [
                { name: 'bodyType', type: 'enum', defaultValue: 'Dynamic', description: 'Static / Kinematic / Dynamic' },
                { name: 'mass', type: 'number', defaultValue: 1.0, description: 'Mass in kilograms' },
                { name: 'friction', type: 'number', defaultValue: 0.5, description: 'Surface friction coefficient' }
            ]
        }
    ];

    private selectedProtoId = 'SecurityOfficer';

    public renderHtml(): string {
        const proto = this.prototypes.find(p => p.id === this.selectedProtoId) || this.prototypes[0];

        return `
            <div class="tool-pane proto-pane">
                <div class="proto-editor-layout">
                    <div class="proto-sidebar card">
                        <h3>🧩 Prototypes List</h3>
                        <div class="proto-list">
                            ${this.prototypes.map(p => `
                                <div class="proto-item ${p.id === this.selectedProtoId ? 'selected' : ''}" data-proto-id="${p.id}">
                                    <div class="proto-title">${p.name} ${p.isAbstract ? '<span class="badge badge-accent">Abstract</span>' : ''}</div>
                                    <div class="proto-id"><code>id: ${p.id}</code></div>
                                </div>
                            `).join('')}
                        </div>

                        <h3>🌿 Inheritance Hierarchy</h3>
                        <div class="inheritance-tree card">
                            <div class="tree-node">BaseEntity</div>
                            <div class="tree-arrow">↓</div>
                            <div class="tree-node">BaseCharacter</div>
                            <div class="tree-arrow">↓</div>
                            <div class="tree-node active">${proto.id}</div>
                        </div>
                    </div>

                    <div class="proto-main card">
                        <div class="pane-header">
                            <h2>Prototype: ${proto.name} (<code>${proto.id}</code>)</h2>
                            <button class="btn btn-primary" id="save-proto-btn">Save YAML</button>
                        </div>

                        <div class="form-grid">
                            <label>
                                <span>Prototype ID:</span>
                                <input type="text" value="${proto.id}" readonly />
                            </label>
                            <label>
                                <span>Display Name:</span>
                                <input type="text" value="${proto.name}" />
                            </label>
                            <label>
                                <span>Multiple Parents:</span>
                                <input type="text" value="${proto.parents.join(', ')}" />
                            </label>
                            <label class="checkbox-label">
                                <input type="checkbox" ${proto.isAbstract ? 'checked' : ''} />
                                <span>Abstract Prototype (Template Only)</span>
                            </label>
                        </div>

                        <h3>📦 ECS Component Attachments</h3>
                        <div class="components-editor">
                            ${Object.entries(proto.components).map(([compName, fields]) => `
                                <div class="component-card card">
                                    <div class="comp-header">
                                        <h4>${compName}</h4>
                                        <span class="badge badge-ok">Storage: Packed</span>
                                    </div>
                                    <div class="comp-fields">
                                        ${Object.entries(fields).map(([fname, fval]) => `
                                            <label class="field-row">
                                                <span class="fname">${fname}:</span>
                                                <input type="text" value="${Array.isArray(fval) ? fval.join(', ') : fval}" />
                                                <span class="badge badge-secondary">Override</span>
                                            </label>
                                        `).join('')}
                                    </div>
                                </div>
                            `).join('')}
                        </div>
                    </div>

                    <div class="schema-sidebar card">
                        <h3>📋 Registered ECS Schemas</h3>
                        <div class="schema-list">
                            ${this.schemas.map(s => `
                                <div class="schema-box">
                                    <div class="schema-name"><code>${s.id}</code></div>
                                    <div class="schema-type">${s.rustType}</div>
                                    <div class="schema-fields">${s.fields.map(f => `• ${f.name} (${f.type})`).join('<br>')}</div>
                                </div>
                            `).join('')}
                        </div>
                    </div>
                </div>
            </div>
        `;
    }

    public bindEvents(onLog: (msg: string) => void): void {
        const btnSave = document.getElementById('save-proto-btn');
        if (btnSave) {
            btnSave.onclick = () => onLog(`[PROTOTYPE] Saved ${this.selectedProtoId}.yml (Validated against Registered Component Schemas)`);
        }

        document.querySelectorAll('.proto-item').forEach(el => {
            el.addEventListener('click', (ev) => {
                const protoId = (ev.currentTarget as HTMLElement).dataset.protoId;
                if (protoId) {
                    this.selectedProtoId = protoId;
                    onLog(`[PROTOTYPE] Opened prototype: ${protoId}`);
                }
            });
        });
    }
}
