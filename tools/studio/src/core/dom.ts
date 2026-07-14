export function required<T extends Element>(selector: string, root: ParentNode = document): T {
  const element = root.querySelector<T>(selector);
  if (!element) {
    throw new Error(`Missing element: ${selector}`);
  }
  return element;
}

export function clear(element: Element): void {
  element.replaceChildren();
}

export function element<K extends keyof HTMLElementTagNameMap>(
  tag: K,
  options: {
    className?: string;
    text?: string;
    title?: string;
    attrs?: Record<string, string>;
  } = {},
): HTMLElementTagNameMap[K] {
  const result = document.createElement(tag);
  if (options.className) result.className = options.className;
  if (options.text !== undefined) result.textContent = options.text;
  if (options.title) result.title = options.title;
  if (options.attrs) {
    for (const [key, value] of Object.entries(options.attrs)) {
      result.setAttribute(key, value);
    }
  }
  return result;
}

export function button(
  label: string,
  onClick: () => void,
  className = '',
): HTMLButtonElement {
  const result = element('button', { className, text: label });
  result.type = 'button';
  result.addEventListener('click', onClick);
  return result;
}

export function field(
  labelText: string,
  control: HTMLElement,
  hint?: string,
): HTMLLabelElement {
  const wrapper = element('label', { className: 'field' });
  wrapper.append(element('span', { className: 'field-label', text: labelText }), control);
  if (hint) wrapper.append(element('small', { className: 'field-hint', text: hint }));
  return wrapper;
}

export function textInput(value: string, onChange: (value: string) => void): HTMLInputElement {
  const input = element('input');
  input.type = 'text';
  input.value = value;

  let committedValue = value;

  const commitValue = (): void => {
    if (input.value === committedValue) return;
    committedValue = input.value;
    onChange(input.value);
  };

  // Inspector callbacks often rebuild the editor. Committing on every keystroke
  // would remove the focused input after the first character. Keep the browser's
  // native editing session alive and commit when the field is finished instead.
  input.addEventListener('change', commitValue);
  input.addEventListener('keydown', (event) => {
    if (event.key === 'Enter') {
      event.preventDefault();
      input.blur();
    } else if (event.key === 'Escape') {
      event.preventDefault();
      input.value = committedValue;
      input.blur();
    }
  });

  return input;
}

export function numberInput(
  value: number,
  onChange: (value: number) => void,
  options: { min?: number; max?: number; step?: number } = {},
): HTMLInputElement {
  const input = element('input');
  input.type = 'number';
  input.value = String(value);
  if (options.min !== undefined) input.min = String(options.min);
  if (options.max !== undefined) input.max = String(options.max);
  if (options.step !== undefined) input.step = String(options.step);

  let committedValue = value;

  const commitValue = (): void => {
    const parsed = Number(input.value);
    if (!Number.isFinite(parsed) || parsed === committedValue) return;
    committedValue = parsed;
    onChange(parsed);
  };

  input.addEventListener('change', commitValue);
  input.addEventListener('keydown', (event) => {
    if (event.key === 'Enter') {
      event.preventDefault();
      input.blur();
    } else if (event.key === 'Escape') {
      event.preventDefault();
      input.value = String(committedValue);
      input.blur();
    }
  });

  return input;
}

export function selectInput(
  value: string,
  options: readonly string[],
  onChange: (value: string) => void,
): HTMLSelectElement {
  const select = element('select');
  for (const optionValue of options) {
    const option = element('option', { text: optionValue });
    option.value = optionValue;
    option.selected = optionValue === value;
    select.append(option);
  }
  select.addEventListener('change', () => onChange(select.value));
  return select;
}

export function checkboxInput(value: boolean, onChange: (value: boolean) => void): HTMLInputElement {
  const input = element('input');
  input.type = 'checkbox';
  input.checked = value;
  input.addEventListener('change', () => onChange(input.checked));
  return input;
}

export function modal<T>(
  title: string,
  renderBody: (body: HTMLElement, resolve: (value: T) => void, reject: () => void) => void,
): Promise<T> {
  return new Promise<T>((resolve, reject) => {
    const backdrop = element('div', { className: 'modal-backdrop' });
    const dialog = element('section', { className: 'modal' });
    const header = element('header', { className: 'modal-header' });
    header.append(element('h2', { text: title }));
    const close = button('×', () => finishReject(), 'icon-button');
    header.append(close);
    const body = element('div', { className: 'modal-body' });
    dialog.append(header, body);
    backdrop.append(dialog);
    document.body.append(backdrop);

    const finishResolve = (value: T): void => {
      backdrop.remove();
      resolve(value);
    };
    const finishReject = (): void => {
      backdrop.remove();
      reject(new DOMException('Dialog cancelled', 'AbortError'));
    };

    backdrop.addEventListener('mousedown', (event) => {
      if (event.target === backdrop) finishReject();
    });
    renderBody(body, finishResolve, finishReject);
  });
}

export function toast(message: string, tone: 'info' | 'success' | 'error' = 'info'): void {
  let host = document.querySelector<HTMLElement>('#toast-host');
  if (!host) {
    host = element('div', { className: 'toast-host', attrs: { id: 'toast-host' } });
    document.body.append(host);
  }
  const item = element('div', { className: `toast toast-${tone}`, text: message });
  host.append(item);
  setTimeout(() => item.classList.add('toast-visible'), 10);
  setTimeout(() => {
    item.classList.remove('toast-visible');
    setTimeout(() => item.remove(), 220);
  }, 2600);
}

export function deepClone<T>(value: T): T {
  return structuredClone(value);
}
