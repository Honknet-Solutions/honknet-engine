import { describe, expect, it, vi } from 'vitest';
import {
  executeHuiAction,
  listHuiControlSchemas,
  renderHui,
  validateHuiDocument,
  type HuiContext,
  type HuiNode,
} from '@honknet/hui-runtime';

function context(state: Record<string, unknown> = {}): HuiContext {
  return {
    state,
    localize: (key) => ({ title: 'Terminal', buy: 'Buy' })[key] ?? key,
    action: vi.fn(),
    sendMessage: vi.fn(),
    setState: (path, value) => {
      const segments = path.replace(/^\$state\./, '').split('.').filter(Boolean);
      let current = state;
      for (const segment of segments.slice(0, -1)) {
        const next = current[segment];
        if (typeof next !== 'object' || next === null || Array.isArray(next)) current[segment] = {};
        current = current[segment] as Record<string, unknown>;
      }
      const leaf = segments.at(-1);
      if (leaf) current[leaf] = value;
    },
  };
}

describe('shared HUI runtime', () => {
  it('declares all controls shown by the designer', () => {
    const types = new Set(listHuiControlSchemas().map((schema) => schema.type));
    for (const type of [
      'Window', 'Row', 'Column', 'Grid', 'Panel', 'Canvas', 'Overlay',
      'ScrollContainer', 'SplitContainer', 'TabContainer', 'Label', 'Button',
      'Image', 'TextInput', 'Checkbox', 'Slider', 'ProgressBar', 'List',
      'Dropdown', 'EntityView', 'InventoryGrid', 'PaperDoll', 'MapView', 'ChatBox',
    ]) expect(types.has(type)).toBe(true);
  });

  it('validates and renders a representative interface', () => {
    const document: HuiNode = {
      type: 'Window',
      id: 'shop',
      title: 'title',
      width: 640,
      height: 420,
      children: [{
        type: 'Column',
        children: [
          { type: 'Label', id: 'balance', text: '$state.balance' },
          { type: 'List', id: 'products', items: '$state.products' },
          { type: 'Button', id: 'buy', text: 'buy', onClick: { type: 'SendMessage', message: 'buy' } },
        ],
      }],
    };
    expect(validateHuiDocument(document).some((issue) => issue.severity === 'error')).toBe(false);
    const ctx = context({ balance: 250, products: ['Ammo', 'Armor'] });
    const root = renderHui(document, ctx, { document: window.document });
    expect(root.querySelector('[data-hui-id="balance"]')?.textContent).toBe('250');
    expect(root.querySelectorAll('.hui-list-item')).toHaveLength(2);
    (root.querySelector('[data-hui-id="buy"]') as HTMLButtonElement).click();
    expect(ctx.sendMessage).toHaveBeenCalledWith('buy', {});
  });

  it('supports freeform layout and state actions', () => {
    const state: Record<string, unknown> = { overlay: { open: false } };
    const ctx = context(state);
    const document: HuiNode = {
      type: 'Canvas',
      width: 800,
      height: 600,
      children: [{
        type: 'Button',
        id: 'toggle',
        x: 120,
        y: 80,
        width: 160,
        height: 40,
        text: 'Toggle',
        onClick: { type: 'ToggleState', path: '$state.overlay.open' },
      }],
    };
    const root = renderHui(document, ctx, { document: window.document });
    const button = root.querySelector('[data-hui-id="toggle"]') as HTMLButtonElement;
    expect(button.style.left).toBe('120px');
    expect(button.style.top).toBe('80px');
    button.click();
    expect((state.overlay as Record<string, unknown>).open).toBe(true);
  });

  it('executes action sequences in order', () => {
    const calls: string[] = [];
    const state: Record<string, unknown> = {};
    executeHuiAction({
      type: 'Sequence',
      actions: [
        { type: 'SetState', path: '$state.selected', value: 42 },
        { type: 'CallController', action: 'done', arguments: { selected: '$state.selected' } },
      ],
    }, {
      ...context(state),
      action: (name, payload) => calls.push(`${name}:${JSON.stringify(payload)}`),
    });
    expect(state.selected).toBe(42);
    expect(calls).toEqual(['done:{"selected":42}']);
  });
});
