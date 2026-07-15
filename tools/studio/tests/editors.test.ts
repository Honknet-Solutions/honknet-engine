import { readFile } from 'node:fs/promises';
import { resolve } from 'node:path';
import { describe, expect, it } from 'vitest';

import type { ProjectMetadata } from '../src/core/types';
import { BehaviorEditor } from '../src/editors/behaviorEditor';
import { MapEditor } from '../src/editors/mapEditor';
import { SchemaEditor } from '../src/editors/schemaEditor';
import { StateMachineEditor } from '../src/editors/stateMachineEditor';
import { UiEditor } from '../src/editors/uiEditor';

const root = resolve(import.meta.dirname, '../../..');
const metadata: ProjectMetadata = {
  prototypes: ['DebugPlayer', 'DebugDoor', 'DebugWrench'],
  prototypeSummaries: [
    { id: 'DebugPlayer', abstract: false },
    { id: 'DebugDoor', abstract: false },
    { id: 'DebugWrench', abstract: false },
  ],
  componentSchemas: [],
  localizationKeys: ['example-status-title', 'example-status-ping'],
  assets: [],
  assetSummaries: [],
  rsiDirectories: [],
  tiles: [
    { id: 'floor', label: 'Floor', color: '#333333', collision: false, category: 'Floor' },
    { id: 'wall', label: 'Wall', color: '#999999', collision: true, category: 'Structure' },
  ],
};

async function source(relative: string): Promise<string> {
  return readFile(resolve(root, relative), 'utf8');
}

function errors(editor: { validate(): Array<{ severity: string }> }): number {
  return editor.validate().filter((message) => message.severity === 'error').length;
}

describe('Studio document editors', () => {
  it('round-trips the example map', async () => {
    const editor = new MapEditor(await source('examples/minimal-game/maps/debug-map.yml'), 'debug-map.yml', metadata);
    expect(errors(editor)).toBe(0);
    const roundTrip = new MapEditor(editor.serialize(), 'debug-map.yml', metadata);
    expect(errors(roundTrip)).toBe(0);
  });

  it('round-trips HUI through the shared schema', async () => {
    const editor = new UiEditor(await source('examples/minimal-game/content/ui/example-status.hui.yml'), 'example-status.hui.yml', metadata);
    expect(errors(editor)).toBe(0);
    const roundTrip = new UiEditor(editor.serialize(), 'example-status.hui.yml', metadata);
    expect(errors(roundTrip)).toBe(0);
  });

  it('round-trips behavior graphs with stable editor IDs', async () => {
    const editor = new BehaviorEditor(await source('examples/minimal-game/content/behaviors/example-door.hgraph.yml'), 'example-door.hgraph.yml');
    expect(errors(editor)).toBe(0);
    const serialized = editor.serialize();
    expect(serialized).toContain('editorId:');
    expect(errors(new BehaviorEditor(serialized, 'example-door.hgraph.yml'))).toBe(0);
  });

  it('round-trips state machines and validates transitions', async () => {
    const editor = new StateMachineEditor(await source('examples/minimal-game/content/state-machines/example-door.hsm.yml'), 'example-door.hsm.yml');
    expect(errors(editor)).toBe(0);
    expect(errors(new StateMachineEditor(editor.serialize(), 'example-door.hsm.yml'))).toBe(0);
  });

  it('creates valid component schemas', () => {
    const editor = SchemaEditor.create('Armor');
    expect(errors(editor)).toBe(0);
    expect(editor.serialize()).toContain('type: component-schema');
  });
});
