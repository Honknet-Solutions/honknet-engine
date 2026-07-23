import { ProfilerFrame } from '../types';

export class ProfilerTool {
    private currentFrame: ProfilerFrame = {
        tps: 30.0,
        frameTimeMs: 14.2,
        entitiesCount: 1240,
        physicsContacts: 38,
        memoryMb: 84.5,
        timings: [
            { name: 'PhysicsSystem', phase: 'Physics', durationMs: 4.8, cpuPercentage: 33.8 },
            { name: 'ReplicationBuildSystem', phase: 'ReplicationPrepare', durationMs: 3.2, cpuPercentage: 22.5 },
            { name: 'PvsVisibilitySystem', phase: 'ReplicationPrepare', durationMs: 2.4, cpuPercentage: 16.9 },
            { name: 'MovementSystem', phase: 'Simulation', durationMs: 1.6, cpuPercentage: 11.2 },
            { name: 'TileMapRenderMeshJob', phase: 'Frame', durationMs: 1.2, cpuPercentage: 8.5 },
            { name: 'PersistenceJournalCommit', phase: 'Persistence', durationMs: 1.0, cpuPercentage: 7.1 }
        ]
    };

    public renderHtml(): string {
        return `
            <div class="tool-pane profiler-pane">
                <div class="pane-header">
                    <h2>⚡ Profiler & Performance Inspector</h2>
                    <span class="badge badge-ok">Tick Rate: 30.0 TPS (p99 = 14.2ms)</span>
                </div>

                <div class="metrics-summary-grid">
                    <div class="metric-card card">
                        <div class="metric-val text-emerald">${this.currentFrame.tps} TPS</div>
                        <div class="metric-label">Server Tick Rate</div>
                    </div>
                    <div class="metric-card card">
                        <div class="metric-val text-cyan">${this.currentFrame.frameTimeMs} ms</div>
                        <div class="metric-label">Tick Duration (Target < 33.3ms)</div>
                    </div>
                    <div class="metric-card card">
                        <div class="metric-val text-indigo">${this.currentFrame.entitiesCount}</div>
                        <div class="metric-label">Active ECS Entities</div>
                    </div>
                    <div class="metric-card card">
                        <div class="metric-val text-amber">${this.currentFrame.physicsContacts}</div>
                        <div class="metric-label">Active Physics Contacts</div>
                    </div>
                    <div class="metric-card card">
                        <div class="metric-val text-purple">${this.currentFrame.memoryMb} MB</div>
                        <div class="metric-label">Heap Memory RSS</div>
                    </div>
                </div>

                <div class="card">
                    <h3>⏱️ System Execution Timings Breakdown</h3>
                    <div class="timings-list">
                        ${this.currentFrame.timings.map(t => `
                            <div class="timing-row">
                                <div class="timing-name">
                                    <b>${t.name}</b>
                                    <span class="badge badge-secondary">${t.phase}</span>
                                </div>
                                <div class="timing-bar-wrapper">
                                    <div class="timing-bar" style="width: ${t.cpuPercentage * 2.5}%"></div>
                                </div>
                                <div class="timing-duration"><code>${t.durationMs} ms</code> (${t.cpuPercentage}%)</div>
                            </div>
                        `).join('')}
                    </div>
                </div>
            </div>
        `;
    }
}
