import YAML from 'yaml';
import './style.css';

type StudioDocument = 'prototype' | 'component-schema' | 'hui' | 'behavior' | 'map' | 'localization';

const templates: Record<StudioDocument, string> = {
  prototype: `- type: entity\n  id: NewEntity\n  name: new-entity-name\n  components:\n    - type: Transform\n    - type: Sprite\n      layers:\n        - map: base\n          texture: /Resources/Textures/example.png\n`,
  'component-schema': `type: component-schema\nid: NewComponent\nreplication:\n  mode: server-to-client\nfields:\n  value:\n    type: float\n    default: 0\n`,
  hui: `type: Window\nid: example.window\ntitle: example-window-title\nchildren:\n  - type: Button\n    text: example-action\n    onClick: action\n`,
  behavior: `id: example-behavior\nevents:\n  OnInteract:\n    - node: Log\n      level: info\n      message: Interacted\n`,
  map: `map:\n  id: new-map\n  grids:\n    - id: main\n      chunks: []\n  entities: []\n`,
  localization: `new-entity-name = новая сущность\n`,
};

const root = document.querySelector<HTMLDivElement>('#app');
if (!root) throw new Error('Missing #app');
root.innerHTML = `
  <header><div><p>HONKNET SOLUTIONS</p><h1>Honknet Studio</h1></div><div class="actions"><button id="open">Open</button><button id="save">Save</button><button id="download">Download</button></div></header>
  <main>
    <aside>
      <h2>Builder</h2>
      <button data-kind="prototype">Prototype</button>
      <button data-kind="component-schema">Component Schema</button>
      <button data-kind="hui">UI (HUI)</button>
      <button data-kind="behavior">Behavior Graph</button>
      <button data-kind="map">Map</button>
      <button data-kind="localization">Localization</button>
      <h2>Validation</h2>
      <pre id="validation">Ready.</pre>
    </aside>
    <section class="workspace">
      <textarea id="source" spellcheck="false"></textarea>
      <section id="preview"></section>
    </section>
  </main>
  <input id="file" type="file" hidden>
`;

const source = required<HTMLTextAreaElement>('#source');
const preview = required<HTMLElement>('#preview');
const validation = required<HTMLElement>('#validation');
const fileInput = required<HTMLInputElement>('#file');
let currentKind: StudioDocument = 'prototype';
let currentHandle: FileSystemFileHandle | null = null;
source.value = templates[currentKind];

for (const button of document.querySelectorAll<HTMLButtonElement>('[data-kind]')) {
  button.addEventListener('click', () => {
    currentKind = button.dataset.kind as StudioDocument;
    source.value = templates[currentKind];
    currentHandle = null;
    update();
  });
}
source.addEventListener('input', update);
required<HTMLButtonElement>('#open').addEventListener('click', () => fileInput.click());
fileInput.addEventListener('change', async () => {
  const file = fileInput.files?.[0];
  if (!file) return;
  source.value = await file.text();
  update();
});
required<HTMLButtonElement>('#save').addEventListener('click', async () => {
  const picker = window.showSaveFilePicker;
  if (!picker) return download();
  currentHandle ??= await picker({ suggestedName: suggestedName() });
  const writable = await currentHandle.createWritable();
  await writable.write(source.value);
  await writable.close();
});
required<HTMLButtonElement>('#download').addEventListener('click', download);

function update(): void {
  try {
    const parsed = currentKind === 'localization' ? source.value : YAML.parse(source.value);
    validation.textContent = 'Valid';
    preview.replaceChildren(renderPreview(parsed));
  } catch (error) {
    validation.textContent = error instanceof Error ? error.message : String(error);
    preview.textContent = 'Preview unavailable.';
  }
}

function renderPreview(value: unknown): HTMLElement {
  const container = document.createElement('div');
  container.className = 'preview-card';
  const title = document.createElement('h2');
  title.textContent = currentKind.replace('-', ' ').toUpperCase();
  const pre = document.createElement('pre');
  pre.textContent = typeof value === 'string' ? value : JSON.stringify(value, null, 2);
  container.append(title, pre);
  return container;
}

function download(): void {
  const blob = new Blob([source.value], { type: 'text/plain;charset=utf-8' });
  const anchor = document.createElement('a');
  anchor.href = URL.createObjectURL(blob);
  anchor.download = suggestedName();
  anchor.click();
  URL.revokeObjectURL(anchor.href);
}

function suggestedName(): string {
  return currentKind === 'localization' ? 'messages.ftl' : `${currentKind}.yml`;
}

function required<T extends Element>(selector: string): T {
  const element = document.querySelector<T>(selector);
  if (!element) throw new Error(`Missing ${selector}`);
  return element;
}

update();

declare global {
  interface Window {
    showSaveFilePicker?: (options?: { suggestedName?: string }) => Promise<FileSystemFileHandle>;
  }
}
