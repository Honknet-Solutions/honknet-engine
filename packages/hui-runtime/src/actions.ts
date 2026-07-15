import { resolveHuiObject, resolveHuiValue, setPath } from './bindings';
import type { HuiAction, HuiContext } from './types';

export function executeHuiAction(action: HuiAction | undefined, context: HuiContext, eventValue?: unknown): void {
  if (!action) return;

  if (typeof action === 'string') {
    context.action(action, eventValue);
    return;
  }

  switch (action.type) {
    case 'SendMessage': {
      const payload = resolveArguments(action.arguments, context, eventValue);
      if (context.sendMessage) context.sendMessage(action.message, payload);
      else context.action(action.message, payload);
      break;
    }
    case 'CallController': {
      context.action(action.action, resolveArguments(action.arguments, context, eventValue));
      break;
    }
    case 'SetState': {
      const value = resolveHuiObject(action.value, context.state, eventValue);
      if (context.setState) context.setState(action.path, value);
      else setPath(context.state, action.path, value);
      break;
    }
    case 'ToggleState': {
      const current = Boolean(resolveHuiValue(action.path, context.state, eventValue));
      if (context.setState) context.setState(action.path, !current);
      else setPath(context.state, action.path, !current);
      break;
    }
    case 'OpenWindow':
      context.openWindow?.(action.window);
      break;
    case 'CloseWindow':
      context.closeWindow?.(action.window);
      break;
    case 'PlaySound':
      context.playSound?.(action.source);
      break;
    case 'Sequence':
      for (const child of action.actions) executeHuiAction(child, context, eventValue);
      break;
  }
}

function resolveArguments(
  argumentsValue: Record<string, unknown> | undefined,
  context: HuiContext,
  eventValue: unknown,
): Record<string, unknown> {
  if (!argumentsValue) return eventValue === undefined ? {} : { value: eventValue };
  return resolveHuiObject(argumentsValue, context.state, eventValue) as Record<string, unknown>;
}
