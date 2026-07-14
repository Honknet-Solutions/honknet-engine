import type { JsonValue, UiSessionState } from '@honknet/shared';

export type UiActionSender = (sessionId: string, action: string, payload: JsonValue) => void;
export type UiFactory = (state: UiSessionState, send: UiActionSender) => HTMLElement;

export class UiRegistry {
  private readonly factories = new Map<string, UiFactory>();

  register(key: string, factory: UiFactory): void {
    if (this.factories.has(key)) {
      throw new Error(`Duplicate UI key ${key}`);
    }
    this.factories.set(key, factory);
  }

  create(state: UiSessionState, send: UiActionSender): HTMLElement {
    const factory = this.factories.get(state.key);
    if (!factory) {
      throw new Error(`Unknown UI key ${state.key}`);
    }
    return factory(state, send);
  }
}

export type ClientGameModule = Readonly<{
  id: string;
  register: (context: { ui: UiRegistry }) => void;
}>;

export function defineClientModule(module: ClientGameModule): ClientGameModule {
  return module;
}
