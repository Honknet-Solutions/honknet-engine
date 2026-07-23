import { MapData, TileDefinition } from '../types';

export class MapEditorTool {
    private selectedTool: 'pencil' | 'eraser' | 'rect' | 'fill' | 'eyedropper' = 'pencil';
    private activeTileId = 'floor';
    private activeLayer = 'floor';
    private showCollision = true;

    private mapData: MapData = {
        id: 'SpaceStationAlpha',
        tileSize: 1.0,
        tiles: [
            { id: 'space', name: 'Space Vacuum', solid: false, friction: 0.0, color: '#090d16' },
            { id: 'floor', name: 'Steel Floor', solid: false, friction: 0.8, color: '#2a3d56' },
            { id: 'wall', name: 'Reinforced Wall', solid: true, friction: 1.0, color: '#64748b' },
            { id: 'glass', name: 'Window Glass', solid: true, friction: 0.5, color: '#38bdf8' },
            { id: 'hazard', name: 'Hazard Stripe', solid: false, friction: 0.8, color: '#eab308' }
        ],
        chunks: [
            {
                x: 0,
                y: 0,
                revision: 1,
                tiles: Array(16 * 16).fill(0)
            }
        ],
        entities: [
            { id: 'spawner_1', prototypeId: 'PlayerSpawn', x: 4, y: 4, layer: 'objects' },
            { id: 'door_1', prototypeId: 'AirlockDoor', x: 8, y: 4, layer: 'objects' }
        ]
    };

    constructor() {
        // Initialize sample grid with walls around perimeter
        const tiles = this.mapData.chunks[0].tiles;
        for (let x = 0; x < 16; x++) {
            for (let y = 0; y < 16; y++) {
                if (x === 0 || x === 15 || y === 0 || y === 15) {
                    tiles[y * 16 + x] = 2; // wall
                } else {
                    tiles[y * 16 + x] = 1; // floor
                }
            }
        }
    }

    public getMapData(): MapData {
        return this.mapData;
    }

    public renderHtml(): string {
        return `
            <div class="tool-pane map-pane">
                <div class="toolbar">
                    <div class="tool-group">
                        <button class="tool-btn ${this.selectedTool === 'pencil' ? 'active' : ''}" id="tool-pencil" title="Pencil Brush (P)">✏️ Pencil</button>
                        <button class="tool-btn ${this.selectedTool === 'eraser' ? 'active' : ''}" id="tool-eraser" title="Eraser (E)">🧹 Eraser</button>
                        <button class="tool-btn ${this.selectedTool === 'rect' ? 'active' : ''}" id="tool-rect" title="Rectangle (R)">🔲 Rectangle</button>
                        <button class="tool-btn ${this.selectedTool === 'fill' ? 'active' : ''}" id="tool-fill" title="Flood Fill (F)">🪣 Fill</button>
                        <button class="tool-btn ${this.selectedTool === 'eyedropper' ? 'active' : ''}" id="tool-eyedropper" title="Eyedropper (I)">🧪 Picker</button>
                    </div>

                    <div class="tool-group">
                        <label>
                            <span>Layer:</span>
                            <select id="map-layer-select">
                                <option value="floor" ${this.activeLayer === 'floor' ? 'selected' : ''}>Ground / Floor</option>
                                <option value="objects" ${this.activeLayer === 'objects' ? 'selected' : ''}>Objects & Anchors</option>
                                <option value="collision" ${this.activeLayer === 'collision' ? 'selected' : ''}>Collision Masks</option>
                            </select>
                        </label>
                        <label class="checkbox-label">
                            <input type="checkbox" id="toggle-collision" ${this.showCollision ? 'checked' : ''} />
                            <span>Show Collision Overlay</span>
                        </label>
                    </div>
                </div>

                <div class="map-editor-layout">
                    <div class="palette-sidebar card">
                        <h3>🎨 Tile Palette</h3>
                        <div class="tile-list">
                            ${this.mapData.tiles.map(t => `
                                <div class="tile-item ${t.id === this.activeTileId ? 'selected' : ''}" data-tile-id="${t.id}">
                                    <span class="tile-swatch" style="background: ${t.color}"></span>
                                    <div class="tile-info">
                                        <div class="tile-name">${t.name}</div>
                                        <div class="tile-meta">${t.solid ? '🧱 Solid' : '🏃 Passable'} | µ=${t.friction}</div>
                                    </div>
                                </div>
                            `).join('')}
                        </div>

                        <h3>🤖 Entity Prototypes</h3>
                        <div class="entity-palette">
                            <div class="entity-item" data-proto="PlayerSpawn">🚩 Player Spawn</div>
                            <div class="entity-item" data-proto="AirlockDoor">🚪 Airlock Door</div>
                            <div class="entity-item" data-proto="ConsoleServer">🖥️ Mainframe Console</div>
                            <div class="entity-item" data-proto="EmergencyLight">🚨 Warning Lamp</div>
                        </div>
                    </div>

                    <div class="viewport-area">
                        <canvas id="map-canvas" width="640" height="640"></canvas>
                        <div class="viewport-overlay">
                            <span>Chunk (0,0) | Rev: ${this.mapData.chunks[0].revision} | Size: 16x16</span>
                        </div>
                    </div>
                </div>
            </div>
        `;
    }

    public bindCanvas(canvas: HTMLCanvasElement, onLog: (msg: string) => void): void {
        const ctx = canvas.getContext('2d');
        if (!ctx) return;

        const draw = () => {
            const tileSize = canvas.width / 16;
            ctx.clearRect(0, 0, canvas.width, canvas.height);

            const chunk = this.mapData.chunks[0];
            for (let x = 0; x < 16; x++) {
                for (let y = 0; y < 16; y++) {
                    const tileIdx = chunk.tiles[y * 16 + x];
                    const tileDef = this.mapData.tiles[tileIdx] || this.mapData.tiles[0];

                    ctx.fillStyle = tileDef.color;
                    ctx.fillRect(x * tileSize, y * tileSize, tileSize - 1, tileSize - 1);

                    // Grid lines
                    ctx.strokeStyle = '#1e293b';
                    ctx.lineWidth = 0.5;
                    ctx.strokeRect(x * tileSize, y * tileSize, tileSize, tileSize);

                    // Collision overlay
                    if (this.showCollision && tileDef.solid) {
                        ctx.fillStyle = 'rgba(239, 68, 68, 0.25)';
                        ctx.fillRect(x * tileSize, y * tileSize, tileSize, tileSize);
                        ctx.strokeStyle = 'rgba(239, 68, 68, 0.8)';
                        ctx.lineWidth = 1;
                        ctx.strokeRect(x * tileSize + 2, y * tileSize + 2, tileSize - 4, tileSize - 4);
                    }
                }
            }

            // Draw placed entities
            for (const entity of this.mapData.entities) {
                ctx.fillStyle = '#10b981';
                ctx.beginPath();
                ctx.arc((entity.x + 0.5) * tileSize, (entity.y + 0.5) * tileSize, tileSize * 0.35, 0, Math.PI * 2);
                ctx.fill();
                ctx.strokeStyle = '#ecfdf5';
                ctx.lineWidth = 1.5;
                ctx.stroke();

                ctx.fillStyle = '#ffffff';
                ctx.font = '10px sans-serif';
                ctx.fillText(entity.prototypeId, (entity.x + 0.1) * tileSize, (entity.y + 0.85) * tileSize);
            }
        };

        draw();

        let isDrawing = false;
        const paintAt = (e: MouseEvent) => {
            const rect = canvas.getBoundingClientRect();
            const scaleX = canvas.width / rect.width;
            const scaleY = canvas.height / rect.height;
            const cx = Math.floor(((e.clientX - rect.left) * scaleX) / (canvas.width / 16));
            const cy = Math.floor(((e.clientY - rect.top) * scaleY) / (canvas.height / 16));

            if (cx >= 0 && cx < 16 && cy >= 0 && cy < 16) {
                const chunk = this.mapData.chunks[0];
                const tileIdx = this.mapData.tiles.findIndex(t => t.id === this.activeTileId);

                if (this.selectedTool === 'eraser') {
                    chunk.tiles[cy * 16 + cx] = 0;
                } else if (this.selectedTool === 'pencil') {
                    chunk.tiles[cy * 16 + cx] = tileIdx >= 0 ? tileIdx : 1;
                }

                chunk.revision++;
                draw();
                onLog(`[MAP] Tile edited at (${cx}, ${cy}) -> ${this.activeTileId} (Rev ${chunk.revision})`);
            }
        };

        canvas.onmousedown = (e) => {
            isDrawing = true;
            paintAt(e);
        };
        canvas.onmousemove = (e) => {
            if (isDrawing) paintAt(e);
        };
        window.onmouseup = () => {
            isDrawing = false;
        };

        // Palette click events
        document.querySelectorAll('.tile-item').forEach(el => {
            el.addEventListener('click', (ev) => {
                const tileId = (ev.currentTarget as HTMLElement).dataset.tileId;
                if (tileId) {
                    this.activeTileId = tileId;
                    document.querySelectorAll('.tile-item').forEach(i => i.classList.remove('selected'));
                    (ev.currentTarget as HTMLElement).classList.add('selected');
                    onLog(`[PALETTE] Selected tile: ${tileId}`);
                }
            });
        });

        // Tool buttons
        ['pencil', 'eraser', 'rect', 'fill', 'eyedropper'].forEach(t => {
            const btn = document.getElementById(`tool-${t}`);
            if (btn) {
                btn.onclick = () => {
                    this.selectedTool = t as any;
                    document.querySelectorAll('.tool-btn').forEach(b => b.classList.remove('active'));
                    btn.classList.add('active');
                };
            }
        });

        const toggleColl = document.getElementById('toggle-collision') as HTMLInputElement;
        if (toggleColl) {
            toggleColl.onchange = () => {
                this.showCollision = toggleColl.checked;
                draw();
            };
        }
    }
}
