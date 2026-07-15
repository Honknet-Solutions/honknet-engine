const BINDING_PREFIXES = ['$state.', '$event.', '$selected.'] as const;

export function isHuiBinding(value: unknown): value is string {
  return typeof value === 'string' && BINDING_PREFIXES.some((prefix) => value.startsWith(prefix));
}

export function resolveHuiValue<T>(value: T | string | undefined, state: Record<string, unknown>, eventValue?: unknown): T | undefined {
  if (value === undefined || !isHuiBinding(value)) {
    return value as T | undefined;
  }

  if (value.startsWith('$event.')) {
    return getPath(eventValue, value.slice('$event.'.length)) as T | undefined;
  }

  if (value.startsWith('$selected.')) {
    return getPath(state.selected, value.slice('$selected.'.length)) as T | undefined;
  }

  return getPath(state, value.slice('$state.'.length)) as T | undefined;
}

export function resolveHuiObject(value: unknown, state: Record<string, unknown>, eventValue?: unknown): unknown {
  if (isHuiBinding(value)) {
    return resolveHuiValue(value, state, eventValue);
  }
  if (Array.isArray(value)) {
    return value.map((entry) => resolveHuiObject(entry, state, eventValue));
  }
  if (isRecord(value)) {
    return Object.fromEntries(Object.entries(value).map(([key, entry]) => [key, resolveHuiObject(entry, state, eventValue)]));
  }
  return value;
}

export function setPath(target: Record<string, unknown>, path: string, value: unknown): void {
  const normalized = path.replace(/^\$state\./, '');
  const segments = normalized.split('.').filter(Boolean);
  if (segments.length === 0) return;

  let cursor: Record<string, unknown> = target;
  for (let index = 0; index < segments.length - 1; index += 1) {
    const segment = segments[index];
    if (!segment) continue;
    const existing = cursor[segment];
    if (!isRecord(existing)) {
      cursor[segment] = {};
    }
    cursor = cursor[segment] as Record<string, unknown>;
  }

  const finalSegment = segments.at(-1);
  if (finalSegment) cursor[finalSegment] = value;
}

export function listBindingPaths(value: unknown, prefix = '$state', maximumDepth = 8): string[] {
  const result: string[] = [];
  const visit = (current: unknown, path: string, depth: number): void => {
    if (depth > maximumDepth) return;
    result.push(path);
    if (Array.isArray(current)) {
      if (current.length > 0) visit(current[0], `${path}.0`, depth + 1);
      return;
    }
    if (!isRecord(current)) return;
    for (const [key, child] of Object.entries(current)) {
      visit(child, `${path}.${key}`, depth + 1);
    }
  };
  visit(value, prefix, 0);
  return result;
}

function getPath(value: unknown, path: string): unknown {
  if (path.length === 0) return value;
  let cursor = value;
  for (const segment of path.split('.').filter(Boolean)) {
    if (Array.isArray(cursor) && /^\d+$/.test(segment)) {
      cursor = cursor[Number(segment)];
      continue;
    }
    if (!isRecord(cursor)) return undefined;
    cursor = cursor[segment];
  }
  return cursor;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}
