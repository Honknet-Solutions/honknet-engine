import { executeHuiAction } from './actions';
import { resolveHuiValue } from './bindings';
import { installHuiStyles } from './styles';
import type { HuiContext, HuiNode, HuiRenderOptions } from './types';

export function renderHui(node: HuiNode, context: HuiContext, options: HuiRenderOptions = {}): HTMLElement {
  const documentValue = options.document ?? document;
  installHuiStyles(documentValue);
  const result = renderNode(node, context, options, documentValue);
  result.classList.add('hui-root');
  return result;
}

function renderNode(node: HuiNode, context: HuiContext, options: HuiRenderOptions, documentValue: Document): HTMLElement {
  const element = createElement(node, context, options, documentValue);
  element.classList.add('hui-control', `hui-${node.type.toLowerCase()}`);
  element.dataset.huiType = node.type;
  if (node.id) element.dataset.huiId = node.id;
  if (node.class) element.classList.add(...node.class.split(/\s+/).filter(Boolean));
  if (node.styleClass) element.classList.add(...node.styleClass.split(/\s+/).filter(Boolean));
  applyCommonProperties(element, node, context);

  const nodeKey = options.getNodeKey?.(node);
  if (nodeKey) element.dataset.huiNodeKey = nodeKey;

  if (options.designMode) {
    element.classList.add('hui-design-node');
    element.addEventListener('pointerdown', (event) => options.onNodePointerDown?.(node, event, element));
    element.addEventListener('click', (event) => options.onNodeClick?.(node, event, element));
    element.addEventListener('dblclick', (event) => options.onNodeDoubleClick?.(node, event, element));
  }

  options.onNodeCreated?.(node, element);
  return element;
}

function createElement(node: HuiNode, context: HuiContext, options: HuiRenderOptions, documentValue: Document): HTMLElement {
  switch (node.type) {
    case 'Window': {
      const section = documentValue.createElement('section');
      if (node.title !== undefined) {
        const titlebar = documentValue.createElement('header');
        titlebar.className = 'hui-window-titlebar';
        titlebar.textContent = resolveText(node.title, context);
        section.append(titlebar);
      }
      const content = documentValue.createElement('div');
      content.className = 'hui-window-content';
      applyContainerLayout(content, node);
      appendChildren(content, node, context, options, documentValue);
      section.append(content);
      return section;
    }
    case 'Row':
    case 'Column':
    case 'Grid':
    case 'Panel':
    case 'Canvas':
    case 'Overlay':
    case 'ScrollContainer': {
      const container = documentValue.createElement('div');
      appendChildren(container, node, context, options, documentValue);
      markEmptyContainer(container, node, options);
      return container;
    }
    case 'SplitContainer': {
      const container = documentValue.createElement('div');
      const children = node.children ?? [];
      const first = children[0];
      const second = children[1];
      const split = clampNumber(node.split ?? 50, 5, 95);
      if (first) {
        const firstElement = renderNode(first, context, options, documentValue);
        firstElement.style.flex = `0 0 ${split}%`;
        container.append(firstElement);
      }
      if (second) {
        const secondElement = renderNode(second, context, options, documentValue);
        secondElement.style.flex = '1 1 auto';
        container.append(secondElement);
      }
      markEmptyContainer(container, node, options);
      return container;
    }
    case 'TabContainer': {
      const container = documentValue.createElement('div');
      const tabbar = documentValue.createElement('div');
      tabbar.className = 'hui-tabbar';
      const content = documentValue.createElement('div');
      content.className = 'hui-tabcontent';
      const children = node.children ?? [];
      const active = Math.max(0, Math.min(children.length - 1, Number(resolveHuiValue(node.activeTab, context.state) ?? 0)));
      children.forEach((child, index) => {
        const tabButton = documentValue.createElement('button');
        tabButton.type = 'button';
        tabButton.className = `hui-tabbutton${index === active ? ' active' : ''}`;
        tabButton.textContent = resolveText(child.tabTitle ?? child.title ?? child.id ?? `Tab ${index + 1}`, context);
        if (!options.designMode) {
          tabButton.addEventListener('click', () => context.action(`${node.id ?? 'tabs'}:select`, index));
        }
        tabbar.append(tabButton);
        if (index === active) content.append(renderNode(child, context, options, documentValue));
      });
      container.append(tabbar, content);
      markEmptyContainer(content, node, options);
      return container;
    }
    case 'Spacer':
      return documentValue.createElement('div');
    case 'Label': {
      const label = documentValue.createElement('span');
      label.textContent = resolveText(node.text ?? '', context);
      const textAlign = typeof node.textAlign === 'string' ? node.textAlign : undefined;
      if (textAlign) label.style.textAlign = textAlign;
      if (typeof node.fontSize === 'number') label.style.fontSize = `${node.fontSize}px`;
      if (node.wrapText === false) label.style.whiteSpace = 'nowrap';
      return label;
    }
    case 'Button': {
      const button = documentValue.createElement('button');
      button.type = 'button';
      button.className = 'hui-button';
      const iconSource = resolveString(node.icon, context);
      if (iconSource) {
        const image = documentValue.createElement('img');
        image.className = 'hui-button-icon';
        image.alt = '';
        void assignResourceUrl(image, iconSource, context);
        button.append(image);
      }
      const text = documentValue.createElement('span');
      text.textContent = resolveText(node.text ?? '', context);
      button.append(text);
      button.disabled = !resolveBoolean(node.enabled, context, true);
      if (resolveBoolean(node.pressed, context, false)) button.classList.add('pressed');
      if (!options.designMode) {
        button.addEventListener('click', () => executeHuiAction(node.onClick, context));
        button.addEventListener('dblclick', () => executeHuiAction(node.onDoubleClick, context));
      }
      return button;
    }
    case 'Image': {
      const source = resolveString(node.source, context) ?? '';
      if (isRsiSource(source)) {
        const image = documentValue.createElement('div');
        image.className = 'hui-image hui-rsi-image';
        image.setAttribute('role', 'img');
        image.setAttribute('aria-label', resolveText(node.alt ?? node.text ?? node.state ?? '', context));
        void renderRsiImage(image, source, node, context, documentValue);
        return image;
      }
      const image = documentValue.createElement('img');
      image.className = 'hui-image';
      void assignResourceUrl(image, source, context);
      image.alt = resolveText(node.alt ?? node.text ?? '', context);
      image.style.objectFit = node.fit ?? 'contain';
      if (node.keepAspect === false) image.style.aspectRatio = 'auto';
      return image;
    }
    case 'TextInput': {
      const multiline = node.multiline === true;
      const input = multiline ? documentValue.createElement('textarea') : documentValue.createElement('input');
      input.className = multiline ? 'hui-textarea' : 'hui-input';
      if (input instanceof HTMLInputElement) input.type = node.password ? 'password' : 'text';
      input.value = String(resolveHuiValue(node.value, context.state) ?? '');
      input.placeholder = resolveText(node.placeholder ?? '', context);
      input.disabled = !resolveBoolean(node.enabled, context, true);
      if (typeof node.maxLength === 'number') input.maxLength = node.maxLength;
      if (!options.designMode) {
        input.addEventListener('input', () => executeHuiAction(node.onChange, context, input.value));
        input.addEventListener('focus', () => executeHuiAction(node.onFocus, context, input.value));
        input.addEventListener('blur', () => executeHuiAction(node.onBlur, context, input.value));
        input.addEventListener('keydown', (event) => {
          const keyboardEvent = event as KeyboardEvent;
          if (keyboardEvent.key === 'Enter' && (!multiline || keyboardEvent.ctrlKey || keyboardEvent.metaKey)) executeHuiAction(node.onSubmit, context, input.value);
        });
      }
      return input;
    }
    case 'Checkbox': {
      const label = documentValue.createElement('label');
      label.className = 'hui-checkbox';
      const input = documentValue.createElement('input');
      input.type = 'checkbox';
      input.checked = resolveBoolean(node.checked ?? node.value, context, false);
      input.disabled = !resolveBoolean(node.enabled, context, true);
      const text = documentValue.createElement('span');
      text.textContent = resolveText(node.text ?? '', context);
      label.append(input, text);
      if (!options.designMode) input.addEventListener('change', () => executeHuiAction(node.onChange, context, input.checked));
      return label;
    }
    case 'Slider': {
      const input = documentValue.createElement('input');
      input.type = 'range';
      input.className = `hui-slider${node.orientation === 'vertical' ? ' vertical' : ''}`;
      input.min = String(node.minimum ?? 0);
      input.max = String(node.maximum ?? 100);
      input.step = String(node.step ?? 1);
      input.value = String(resolveHuiValue(node.value, context.state) ?? node.minimum ?? 0);
      input.disabled = !resolveBoolean(node.enabled, context, true);
      if (!options.designMode) input.addEventListener('input', () => executeHuiAction(node.onValueChanged ?? node.onChange, context, Number(input.value)));
      return input;
    }
    case 'ProgressBar': {
      const progress = documentValue.createElement('div');
      progress.className = 'hui-progress';
      const minimum = Number(node.minimum ?? 0);
      const maximum = Number(node.maximum ?? 100);
      const value = Number(resolveHuiValue(node.value, context.state) ?? minimum);
      const ratio = maximum === minimum ? 0 : clampNumber((value - minimum) / (maximum - minimum), 0, 1);
      const fill = documentValue.createElement('div');
      fill.className = 'hui-progress-fill';
      fill.style.width = `${ratio * 100}%`;
      progress.append(fill);
      if (node.showValue !== false) {
        const label = documentValue.createElement('span');
        label.className = 'hui-progress-label';
        label.textContent = `${Math.round(ratio * 100)}%`;
        progress.append(label);
      }
      return progress;
    }
    case 'List': {
      const list = documentValue.createElement('div');
      list.className = 'hui-list';
      const items = resolveItems(node.items, context);
      const selected = resolveHuiValue(node.selected, context.state);
      items.forEach((item, index) => {
        const itemButton = documentValue.createElement('button');
        itemButton.type = 'button';
        itemButton.className = `hui-list-item${selected === index || selected === getItemValue(item) ? ' selected' : ''}`;
        itemButton.textContent = getItemLabel(item, context);
        if (!options.designMode) itemButton.addEventListener('click', () => executeHuiAction(node.onSelected, context, getItemValue(item) ?? index));
        list.append(itemButton);
      });
      return list;
    }
    case 'Dropdown': {
      const select = documentValue.createElement('select');
      select.className = 'hui-dropdown';
      const items = resolveItems(node.items, context);
      const selected = resolveHuiValue(node.selected, context.state);
      items.forEach((item, index) => {
        const option = documentValue.createElement('option');
        const value = getItemValue(item) ?? index;
        option.value = String(value);
        option.textContent = getItemLabel(item, context);
        option.selected = selected === value || String(selected ?? '') === String(value);
        select.append(option);
      });
      select.disabled = !resolveBoolean(node.enabled, context, true);
      if (!options.designMode) select.addEventListener('change', () => executeHuiAction(node.onSelected ?? node.onChange, context, select.value));
      return select;
    }
    case 'InventoryGrid': {
      const grid = documentValue.createElement('div');
      grid.className = 'hui-inventory-grid';
      grid.style.gridTemplateColumns = `repeat(${Math.max(1, Math.round(node.columns ?? 6))}, minmax(0, 1fr))`;
      const items = resolveItems(node.items, context);
      items.forEach((item, index) => {
        const slot = documentValue.createElement('button');
        slot.type = 'button';
        slot.className = 'hui-inventory-slot';
        slot.textContent = getItemLabel(item, context);
        if (!options.designMode) slot.addEventListener('click', () => executeHuiAction(node.onSelected, context, getItemValue(item) ?? index));
        grid.append(slot);
      });
      if (items.length === 0) {
        for (let index = 0; index < Math.max(6, node.columns ?? 6); index += 1) {
          const slot = documentValue.createElement('div');
          slot.className = 'hui-inventory-slot';
          grid.append(slot);
        }
      }
      return grid;
    }
    case 'ChatBox': {
      const box = documentValue.createElement('div');
      box.className = 'hui-chatbox';
      const messages = documentValue.createElement('div');
      messages.className = 'hui-chat-messages';
      for (const item of resolveItems(node.items, context)) {
        const row = documentValue.createElement('div');
        row.textContent = getItemLabel(item, context);
        messages.append(row);
      }
      const input = documentValue.createElement('input');
      input.className = 'hui-input hui-chat-input';
      input.placeholder = resolveText(node.placeholder ?? 'chat-message-placeholder', context);
      if (!options.designMode) input.addEventListener('keydown', (event) => {
        if (event.key === 'Enter' && input.value.trim()) {
          executeHuiAction(node.onSubmit, context, input.value);
          input.value = '';
        }
      });
      box.append(messages, input);
      return box;
    }
    case 'EntityView':
    case 'PaperDoll':
    case 'MapView': {
      const view = documentValue.createElement('div');
      view.className = 'hui-game-view';
      const value = resolveHuiValue(node.value, context.state);
      view.textContent = `${node.type}\n${formatPreviewValue(value)}`;
      return view;
    }
    default: {
      const unknown = documentValue.createElement('div');
      unknown.dataset.unknownHuiNode = node.type;
      unknown.textContent = `Unsupported HUI control: ${node.type}`;
      return unknown;
    }
  }
}

async function resolveResourceUrl(source: string, context: HuiContext): Promise<string> {
  if (!context.resolveResource) return source;
  return await context.resolveResource(source);
}

async function assignResourceUrl(image: HTMLImageElement, source: string, context: HuiContext): Promise<void> {
  try {
    image.src = await resolveResourceUrl(source, context);
  } catch {
    image.dataset.resourceError = source;
  }
}

function isRsiSource(source: string): boolean {
  return /\.rsi\/?$/i.test(source);
}

async function renderRsiImage(
  host: HTMLElement,
  source: string,
  node: HuiNode,
  context: HuiContext,
  documentValue: Document,
): Promise<void> {
  try {
    const root = source.replace(/\/$/, '');
    const metaUrl = await resolveResourceUrl(`${root}/meta.json`, context);
    const response = await fetch(metaUrl, { cache: 'no-store' });
    if (!response.ok) throw new Error(`${response.status} ${response.statusText}`);
    const meta = await response.json() as unknown;
    if (!isRecord(meta) || !isRecord(meta.size) || !Array.isArray(meta.states)) throw new Error('Invalid RSI meta.json');
    const frameWidth = Math.max(1, Number(meta.size.x ?? 1));
    const frameHeight = Math.max(1, Number(meta.size.y ?? 1));
    const requestedState = resolveString(node.state, context);
    const state = meta.states.filter(isRecord).find((entry) => entry.name === requestedState)
      ?? meta.states.filter(isRecord)[0];
    if (!state || typeof state.name !== 'string') throw new Error('RSI has no states');
    const pngUrl = await resolveResourceUrl(`${root}/${state.name}.png`, context);
    const probe = documentValue.createElement('img');
    const loaded = new Promise<void>((resolve, reject) => {
      probe.addEventListener('load', () => resolve(), { once: true });
      probe.addEventListener('error', () => reject(new Error(`Failed to load ${state.name}.png`)), { once: true });
    });
    probe.src = pngUrl;
    await loaded;

    const directionCount = Math.max(1, Number(state.directions ?? (Math.floor(probe.naturalHeight / frameHeight) || 1)));
    const direction = Math.max(0, Math.min(directionCount - 1, Math.round(node.rsiDirection ?? 0)));
    const frameCount = Math.max(1, Math.floor(probe.naturalWidth / frameWidth));
    const delays = readRsiDelays(state.delays, direction, frameCount);
    let frame = Math.max(0, Math.min(frameCount - 1, Math.round(node.frame ?? 0)));
    host.style.width ||= `${frameWidth}px`;
    host.style.height ||= `${frameHeight}px`;
    host.style.backgroundImage = `url("${pngUrl.replaceAll('"', '\\"')}")`;
    host.style.backgroundRepeat = 'no-repeat';
    host.style.imageRendering = 'pixelated';

    const draw = (): void => {
      host.style.backgroundSize = `${probe.naturalWidth}px ${probe.naturalHeight}px`;
      host.style.backgroundPosition = `${-frame * frameWidth}px ${-direction * frameHeight}px`;
    };
    draw();
    if (node.animate !== false && frameCount > 1) {
      const advance = (): void => {
        if (!host.isConnected) return;
        const delay = Math.max(0.01, delays[frame] ?? 0.1) * 1000;
        window.setTimeout(() => {
          if (!host.isConnected) return;
          frame = (frame + 1) % frameCount;
          draw();
          advance();
        }, delay);
      };
      advance();
    }
  } catch (error) {
    host.dataset.resourceError = source;
    host.textContent = error instanceof Error ? error.message : String(error);
  }
}

function readRsiDelays(value: unknown, direction: number, frameCount: number): number[] {
  if (!Array.isArray(value) || value.length === 0) return Array.from({ length: frameCount }, () => 0.1);
  if (value.every((entry) => typeof entry === 'number')) return value.map(Number);
  const selected = value[direction] ?? value[0];
  return Array.isArray(selected) ? selected.map((entry) => Math.max(0.01, Number(entry) || 0.1)) : Array.from({ length: frameCount }, () => 0.1);
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

function appendChildren(host: HTMLElement, node: HuiNode, context: HuiContext, options: HuiRenderOptions, documentValue: Document): void {
  for (const child of node.children ?? []) host.append(renderNode(child, context, options, documentValue));
}

function markEmptyContainer(element: HTMLElement, node: HuiNode, options: HuiRenderOptions): void {
  if (options.designMode && (node.children?.length ?? 0) === 0) element.classList.add('hui-design-empty');
}

function applyCommonProperties(element: HTMLElement, node: HuiNode, context: HuiContext): void {
  if (!resolveBoolean(node.visible, context, true)) element.hidden = true;
  const tooltip = resolveString(node.tooltip, context);
  if (tooltip) element.title = resolveText(tooltip, context);

  setDimension(element.style, 'width', node.width ?? node.size?.[0]);
  setDimension(element.style, 'height', node.height ?? node.size?.[1]);
  setPixel(element.style, 'minWidth', node.minWidth);
  setPixel(element.style, 'minHeight', node.minHeight);
  setPixel(element.style, 'maxWidth', node.maxWidth);
  setPixel(element.style, 'maxHeight', node.maxHeight);
  if (node.grow !== undefined) element.style.flexGrow = String(node.grow);
  if (node.shrink !== undefined) element.style.flexShrink = String(node.shrink);
  if (node.margin !== undefined) element.style.margin = spacingToCss(node.margin);
  if (node.type !== 'Window') applyContainerLayout(element, node);
  if (node.opacity !== undefined) element.style.opacity = String(clampNumber(node.opacity, 0, 1));
  if (node.zIndex !== undefined) element.style.zIndex = String(node.zIndex);
  if (node.selfAlign && node.selfAlign !== 'auto') element.style.alignSelf = normalizeAlignment(node.selfAlign);
  if (node.columnSpan !== undefined) element.style.gridColumn = `span ${Math.max(1, Math.round(node.columnSpan))}`;
  if (node.rowSpan !== undefined) element.style.gridRow = `span ${Math.max(1, Math.round(node.rowSpan))}`;

  if (node.type === 'Grid') {
    element.style.gridTemplateColumns = `repeat(${Math.max(1, Math.round(node.columns ?? 2))}, minmax(0, 1fr))`;
    if (node.rows && node.rows > 0) element.style.gridTemplateRows = `repeat(${Math.round(node.rows)}, minmax(0, 1fr))`;
  }
  if (node.type === 'ScrollContainer') element.style.flexDirection = node.orientation === 'horizontal' ? 'row' : 'column';
  if (node.type === 'SplitContainer') element.style.flexDirection = node.orientation === 'vertical' ? 'column' : 'row';

  if (typeof node.x === 'number') element.style.left = `${node.x}px`;
  if (typeof node.y === 'number') element.style.top = `${node.y}px`;
  if (node.anchorLeft === true) element.style.left = `${node.x ?? 0}px`;
  if (node.anchorRight === true) element.style.right = '0px';
  if (node.anchorTop === true) element.style.top = `${node.y ?? 0}px`;
  if (node.anchorBottom === true) element.style.bottom = '0px';
}

function applyContainerLayout(element: HTMLElement, node: HuiNode): void {
  if (node.padding !== undefined) element.style.padding = spacingToCss(node.padding);
  if (node.gap !== undefined) element.style.gap = `${node.gap}px`;
  if (node.rowGap !== undefined) element.style.rowGap = `${node.rowGap}px`;
  if (node.columnGap !== undefined) element.style.columnGap = `${node.columnGap}px`;
  if (node.alignItems) element.style.alignItems = normalizeAlignment(node.alignItems);
  if (node.justifyContent) element.style.justifyContent = normalizeJustify(node.justifyContent);
  if (node.wrap === true) element.style.flexWrap = 'wrap';
  if (node.overflow) element.style.overflow = node.overflow;
  if (node.clip === true) element.style.overflow = 'hidden';
}

function setDimension(style: CSSStyleDeclaration, property: 'width' | 'height', value: number | string | undefined): void {
  if (value === undefined) return;
  style[property] = dimensionToCss(value);
}

function setPixel(style: CSSStyleDeclaration, property: 'minWidth' | 'minHeight' | 'maxWidth' | 'maxHeight', value: number | undefined): void {
  if (value !== undefined) style[property] = `${value}px`;
}

function dimensionToCss(value: number | string): string {
  if (typeof value === 'number') return `${value}px`;
  if (value === 'fill') return '100%';
  if (value === 'auto') return 'auto';
  return value;
}

function spacingToCss(value: number | string): string {
  if (typeof value === 'number') return `${value}px`;
  return value.split(/\s+/).filter(Boolean).map((part) => /^-?\d+(?:\.\d+)?$/.test(part) ? `${part}px` : part).join(' ');
}

function normalizeAlignment(value: string): string {
  return value === 'start' ? 'flex-start' : value === 'end' ? 'flex-end' : value;
}

function normalizeJustify(value: string): string {
  return value === 'start' ? 'flex-start' : value === 'end' ? 'flex-end' : value;
}

function resolveText(value: unknown, context: HuiContext): string {
  const resolved = resolveHuiValue(value as string | undefined, context.state);
  if (resolved == null) return '';
  const text = String(resolved);
  if (typeof value === 'string' && !value.startsWith('$') && /^[A-Za-z0-9_.-]+$/.test(value)) {
    return context.localize(value);
  }
  return text;
}

function resolveString(value: unknown, context: HuiContext): string | undefined {
  const resolved = resolveHuiValue(value as string | undefined, context.state);
  return resolved == null ? undefined : String(resolved);
}

function resolveBoolean(value: unknown, context: HuiContext, fallback: boolean): boolean {
  const resolved = resolveHuiValue(value as boolean | string | undefined, context.state);
  return resolved == null ? fallback : Boolean(resolved);
}

function resolveItems(value: unknown, context: HuiContext): unknown[] {
  const resolved = resolveHuiValue(value as unknown[] | string | undefined, context.state);
  return Array.isArray(resolved) ? resolved : [];
}

function getItemLabel(value: unknown, context: HuiContext): string {
  if (value == null) return '';
  if (typeof value === 'string' || typeof value === 'number' || typeof value === 'boolean') return String(value);
  if (typeof value === 'object') {
    const record = value as Record<string, unknown>;
    const label = record.label ?? record.name ?? record.text ?? record.id;
    if (label !== undefined) return resolveText(label, context);
  }
  return JSON.stringify(value);
}

function getItemValue(value: unknown): unknown {
  if (typeof value === 'object' && value !== null) {
    const record = value as Record<string, unknown>;
    return record.value ?? record.id;
  }
  return value;
}

function formatPreviewValue(value: unknown): string {
  if (value == null) return 'No data';
  if (typeof value === 'string' || typeof value === 'number' || typeof value === 'boolean') return String(value);
  try {
    return JSON.stringify(value, null, 2).slice(0, 180);
  } catch {
    return String(value);
  }
}

function clampNumber(value: number, minimum: number, maximum: number): number {
  return Math.min(maximum, Math.max(minimum, value));
}
