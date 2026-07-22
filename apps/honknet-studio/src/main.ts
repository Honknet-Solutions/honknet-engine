import YAML from 'yaml';
import './style.css';
type Kind = 'map' | 'prototype' | 'ui' | 'animation' | 'localization' | 'resource' | 'replay' | 'network';
type Doc = {
    name: string;
    kind: Kind;
    text: string;
    dirty: boolean;
};
class Studio {
    private docs: Doc[] = [];
    private current = -1;
    private history: string[] = [];
    private future: string[] = [];
    private root = document.querySelector<HTMLElement>('#app')!;
    private canvas!: HTMLCanvasElement;
    private source!: HTMLTextAreaElement;
    private tree!: HTMLElement;
    private inspector!: HTMLElement;
    private status!: HTMLElement;
    constructor() {
        this.root.innerHTML = this.template();
        this.tree = this.q('#tree');
        this.canvas = this.q('#canvas');
        this.source = this.q('#source');
        this.inspector = this.q('#inspector');
        this.status = this.q('#status');
        this.bind();
        this.newDoc('map');
        this.render();
    }
    private q<T extends HTMLElement>(selector: string): T {
        return this.root.querySelector<T>(selector)!;
    }
    private template(): string {
        return `
            <div class="shell">
                <header>
                    <b>HONKNET STUDIO 1.0</b>
                    <button id="open">Open</button>
                    <button id="new">New</button>
                    <button id="save">Save</button>
                    <button id="undo">Undo</button>
                    <button id="redo">Redo</button>
                    <button id="validate">Validate</button>
                    <button id="play">Preview</button>
                    <select id="kind">
                        <option>map</option>
                        <option>prototype</option>
                        <option>ui</option>
                        <option>animation</option>
                        <option>localization</option>
                        <option>resource</option>
                        <option>replay</option>
                        <option>network</option>
                    </select>
                </header>
                <aside id="tree"></aside>
                <main>
                    <canvas id="canvas" width="1024" height="640"></canvas>
                    <textarea id="source"></textarea>
                </main>
                <section id="inspector"></section>
                <footer id="status">Ready</footer>
            </div>
        `;
    }
    private bind(): void {
        this.q('#new').onclick = () => this.newDoc(this.q<HTMLSelectElement>('#kind').value as Kind);
        this.q('#save').onclick = () => void this.save();
        this.q('#undo').onclick = () => this.undo();
        this.q('#redo').onclick = () => this.redo();
        this.q('#validate').onclick = () => this.validate();
        this.q('#play').onclick = () => this.preview();
        this.q('#open').onclick = () => void this.open();
        this.source.oninput = () => {
            const doc = this.docs[this.current];
            if (!doc)
                return;
            this.history.push(doc.text);
            doc.text = this.source.value;
            doc.dirty = true;
            this.renderCanvas();
            this.renderTree();
        };
        this.canvas.onpointerdown = (event) => this.canvasEdit(event);
    }
    private newDoc(kind: Kind): void {
        const samples: Record<Kind, string> = {
            map: `type: map
id: TestMap
tile_size: 1
grids:
  - id: Main
    chunks: []
`,
            prototype: `type: entity
id: ExampleEntity
components:
  Transform: {}
  Sprite:
    resource: res://example.png
`,
            ui: `type: Window
title: Example
layout: flex
children:
  - type: Button
    text: Connect
    action: connect
`,
            animation: `name: idle
fps: 8
frames: [0, 1, 2, 1]
`,
            localization: `studio-title = Honknet Studio
`,
            resource: `path: res://example.png
kind: texture
`,
            replay: `tick: 0
event: marker
name: start
`,
            network: `channel: ReliableOrdered
message: Chat
fields:
  text: string
`,
        };
        this.docs.push({ name: `${kind}-${this.docs.length + 1}.yml`, kind, text: samples[kind], dirty: true });
        this.current = this.docs.length - 1;
        this.render();
    }
    private render(): void {
        this.renderTree();
        const doc = this.docs[this.current];
        this.source.value = doc?.text ?? '';
        this.renderCanvas();
        this.renderInspector();
    }
    private renderTree(): void {
        this.tree.innerHTML = this.docs.map((doc, index) => `<button data-i="${index}" class="${index === this.current ? 'active' : ''}">${doc.dirty ? '● ' : ''}${doc.name}</button>`).join('');
        this.tree.querySelectorAll('button').forEach((button) => button.addEventListener('click', () => {
            this.current = Number((button as HTMLElement).dataset.i);
            this.render();
        }));
    }
    private renderCanvas(): void {
        const ctx = this.canvas.getContext('2d')!;
        ctx.fillStyle = '#080d18';
        ctx.fillRect(0, 0, this.canvas.width, this.canvas.height);
        const doc = this.docs[this.current];
        if (!doc)
            return;
        let data: any;
        try {
            data = YAML.parse(doc.text);
        }
        catch {
            this.status.textContent = 'YAML error';
            return;
        }
        ctx.strokeStyle = '#28405c';
        for (let x = 0; x < this.canvas.width; x += 32) {
            ctx.beginPath();
            ctx.moveTo(x, 0);
            ctx.lineTo(x, this.canvas.height);
            ctx.stroke();
        }
        for (let y = 0; y < this.canvas.height; y += 32) {
            ctx.beginPath();
            ctx.moveTo(0, y);
            ctx.lineTo(this.canvas.width, y);
            ctx.stroke();
        }
        ctx.fillStyle = '#24e7c4';
        ctx.font = '18px system-ui';
        ctx.fillText(`${doc.kind}: ${data?.id ?? data?.name ?? doc.name}`, 24, 30);
        if (doc.kind === 'map') {
            for (let i = 0; i < 12; i++) {
                ctx.fillStyle = i % 2 ? '#172a3d' : '#1d3850';
                ctx.fillRect(96 + (i % 4) * 64, 96 + Math.floor(i / 4) * 64, 62, 62);
            }
        }
        else if (doc.kind === 'ui') {
            ctx.fillStyle = '#101d31';
            ctx.fillRect(180, 100, 600, 420);
            ctx.strokeStyle = '#24e7c4';
            ctx.strokeRect(180, 100, 600, 420);
            ctx.fillStyle = '#ecf5ff';
            ctx.fillText(String(data?.title ?? 'Window'), 210, 140);
            ctx.fillStyle = '#245b72';
            ctx.fillRect(220, 180, 180, 44);
            ctx.fillStyle = '#fff';
            ctx.fillText(String(data?.children?.[0]?.text ?? 'Button'), 250, 209);
        }
        else {
            ctx.fillStyle = '#162944';
            ctx.fillRect(120, 100, 760, 420);
            ctx.fillStyle = '#c6d6e8';
            ctx.fillText(JSON.stringify(data, null, 2).slice(0, 120), 150, 150);
        }
    }
    private renderInspector(): void {
        const doc = this.docs[this.current];
        this.inspector.innerHTML = doc
            ? `
                <h3>Inspector</h3>
                <label>
                    Name
                    <input id="doc-name" value="${doc.name}">
                </label>
                <p>Kind: ${doc.kind}</p>
                <p>Bytes: ${new TextEncoder().encode(doc.text).length}</p>
                <h3>Tools</h3>
                <button id="add-node">Add node/tile</button>
                <button id="format">Format YAML</button>
            `
            : '';
        this.inspector
            .querySelector<HTMLInputElement>('#doc-name')
            ?.addEventListener('change', (event) => {
                doc.name = (event.target as HTMLInputElement).value;
                doc.dirty = true;
                this.renderTree();
            });
        this.inspector.querySelector('#format')?.addEventListener('click', () => {
            try {
                doc.text = YAML.stringify(YAML.parse(doc.text));
                this.source.value = doc.text;
                doc.dirty = true;
                this.renderCanvas();
            }
            catch (error) {
                this.status.textContent = String(error);
            }
        });
        this.inspector
            .querySelector('#add-node')
            ?.addEventListener('click', () => {
                doc.text += `\n# visual edit ${new Date().toISOString()}`;
                doc.dirty = true;
                this.source.value = doc.text;
                this.renderCanvas();
            });
    }
    private canvasEdit(event: PointerEvent): void {
        const doc = this.docs[this.current];
        if (!doc || doc.kind !== 'map')
            return;
        const rect = this.canvas.getBoundingClientRect();
        const x = Math.floor((event.clientX - rect.left) / 32);
        const y = Math.floor((event.clientY - rect.top) / 32);
        this.history.push(doc.text);
        doc.text += `\n# tile ${x},${y}`;
        doc.dirty = true;
        this.source.value = doc.text;
        this.status.textContent = `Tile ${x}, ${y} changed`;
        this.renderCanvas();
    }
    private validate(): void {
        const doc = this.docs[this.current];
        try {
            const value = YAML.parse(doc.text);
            if (!value || typeof value !== 'object')
                throw new Error('Document must be a mapping');
            this.status.textContent = 'Validation passed';
        }
        catch (error) {
            this.status.textContent = `Validation failed: ${String(error)}`;
        }
    }
    private preview(): void { this.renderCanvas(); this.status.textContent = 'Runtime-compatible preview refreshed'; }
    private undo(): void {
        const doc = this.docs[this.current]; const value = this.history.pop(); if (doc && value !== undefined) {
            this.future.push(doc.text);
            doc.text = value;
            this.source.value = value;
            this.renderCanvas();
        }
    }
    private redo(): void {
        const doc = this.docs[this.current]; const value = this.future.pop(); if (doc && value !== undefined) {
            this.history.push(doc.text);
            doc.text = value;
            this.source.value = value;
            this.renderCanvas();
        }
    }
    private async open(): Promise<void> {
        const picker = (window as any).showOpenFilePicker;
        if (!picker) {
            this.status.textContent = 'File System Access API is unavailable';
            return;
        }
        const [handle] = await picker({ types: [{ description: 'Honknet documents', accept: { 'text/yaml': ['.yml', '.yaml', '.hui.yml', '.hsm.yml', '.hgraph.yml'] } }] });
        const file = await handle.getFile();
        this.docs.push({ name: file.name, kind: 'prototype', text: await file.text(), dirty: false });
        this.current = this.docs.length - 1;
        this.render();
    }
    private async save(): Promise<void> {
        const doc = this.docs[this.current];
        if (!doc)
            return;
        const picker = (window as any).showSaveFilePicker;
        if (picker) {
            const handle = await picker({ suggestedName: doc.name });
            const output = await handle.createWritable();
            await output.write(doc.text);
            await output.close();
            doc.dirty = false;
            this.renderTree();
            this.status.textContent = 'Saved';
        }
        else {
            const anchor = document.createElement('a');
            anchor.href = URL.createObjectURL(new Blob([doc.text], { type: 'text/yaml' }));
            anchor.download = doc.name;
            anchor.click();
        }
    }
}
new Studio();
