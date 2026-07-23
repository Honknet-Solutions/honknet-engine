import { PacketInspect, PvsRegion } from '../types';

export class NetworkInspectorTool {
    private packets: PacketInspect[] = [
        { id: 1, kind: 'State', channel: 'UnreliableSequenced', bytes: 142, seq: 1042, ack: 980, timestamp: '10:42:01.102' },
        { id: 2, kind: 'Input', channel: 'UnreliableSequenced', bytes: 48, seq: 1043, ack: 981, timestamp: '10:42:01.135' },
        { id: 3, kind: 'Welcome', channel: 'Control', bytes: 64, seq: 1, ack: 0, timestamp: '10:42:00.000' },
        { id: 4, kind: 'TileMutation', channel: 'ReliableOrdered', bytes: 180, seq: 104, ack: 102, timestamp: '10:41:58.420' }
    ];

    private pvsRegions: PvsRegion[] = [
        { id: 'BridgeRegion', entitiesCount: 420, byteBudgetKbps: 64, active: true },
        { id: 'EngineeringChamber', entitiesCount: 310, byteBudgetKbps: 48, active: true },
        { id: 'CargoBay', entitiesCount: 180, byteBudgetKbps: 32, active: false }
    ];

    public renderHtml(): string {
        return `
            <div class="tool-pane network-pane">
                <div class="pane-header">
                    <h2>🌐 Network Inspector & PVS Spatial Visualizer</h2>
                    <span class="badge badge-accent">WebSocket / QUIC Binary Transport</span>
                </div>

                <div class="card">
                    <h3>📡 Live Binary Packet Inspector Stream</h3>
                    <table class="data-table">
                        <thead>
                            <tr>
                                <th>Timestamp</th>
                                <th>Message Kind</th>
                                <th>Channel</th>
                                <th>Sequence</th>
                                <th>Ack</th>
                                <th>Payload Size</th>
                            </tr>
                        </thead>
                        <tbody>
                            ${this.packets.map(p => `
                                <tr>
                                    <td><code>${p.timestamp}</code></td>
                                    <td><b>${p.kind}</b></td>
                                    <td><span class="badge badge-secondary">${p.channel}</span></td>
                                    <td>#${p.seq}</td>
                                    <td>#${p.ack}</td>
                                    <td>${p.bytes} bytes</td>
                                </tr>
                            `).join('')}
                        </tbody>
                    </table>
                </div>

                <div class="card">
                    <h3>🎯 PVS (Potentially Visible Set) Region Budgets</h3>
                    <div class="target-grid">
                        ${this.pvsRegions.map(r => `
                            <div class="target-box ${r.active ? 'active' : ''}">
                                <div class="target-title">${r.id}</div>
                                <div class="target-desc">${r.entitiesCount} Entities | Budget: ${r.byteBudgetKbps} KB/s</div>
                                <div class="badge ${r.active ? 'badge-ok' : 'badge-secondary'}">${r.active ? 'Active Visible' : 'Culled'}</div>
                            </div>
                        `).join('')}
                    </div>
                </div>
            </div>
        `;
    }
}
