export type HuiNode = {
  type: string;
  id?: string;
  text?: string;
  title?: string;
  source?: string;
  enabled?: boolean;
  visible?: boolean;
  class?: string;
  children?: HuiNode[];
  onClick?: string;
};

export type HuiContext = {
  state: Record<string, unknown>;
  localize: (key: string) => string;
  action: (name: string, payload?: unknown) => void;
};

export function renderHui(node: HuiNode, context: HuiContext): HTMLElement {
  const element = createElement(node, context);
  if (node.id) element.dataset.huiId = node.id;
  if (node.class) element.className = node.class;
  if (node.visible === false) element.hidden = true;
  for (const child of node.children ?? []) {
    element.appendChild(renderHui(child, context));
  }
  return element;
}

function createElement(node: HuiNode, context: HuiContext): HTMLElement {
  switch (node.type) {
    case 'Window': {
      const section = document.createElement('section');
      section.className = 'hui-window';
      if (node.title) {
        const title = document.createElement('h2');
        title.textContent = resolveText(node.title, context);
        section.appendChild(title);
      }
      return section;
    }
    case 'Row': {
      const row = document.createElement('div');
      row.className = 'hui-row';
      return row;
    }
    case 'Column': {
      const column = document.createElement('div');
      column.className = 'hui-column';
      return column;
    }
    case 'Label': {
      const label = document.createElement('span');
      label.textContent = resolveText(node.text ?? '', context);
      return label;
    }
    case 'Image': {
      const image = document.createElement('img');
      image.src = node.source ?? '';
      image.alt = node.text ?? '';
      return image;
    }
    case 'Button': {
      const button = document.createElement('button');
      button.type = 'button';
      button.textContent = resolveText(node.text ?? '', context);
      button.disabled = node.enabled === false;
      if (node.onClick) button.addEventListener('click', () => context.action(node.onClick!));
      return button;
    }
    case 'Input': {
      const input = document.createElement('input');
      input.placeholder = resolveText(node.text ?? '', context);
      return input;
    }
    default: {
      const unknown = document.createElement('div');
      unknown.dataset.unknownHuiNode = node.type;
      return unknown;
    }
  }
}

function resolveText(value: string, context: HuiContext): string {
  if (value.startsWith('$state.')) {
    const key = value.slice('$state.'.length);
    const resolved = context.state[key];
    return resolved == null ? '' : String(resolved);
  }
  return context.localize(value);
}
