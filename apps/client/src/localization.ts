export class LocalizationBundle {
  private readonly messages = new Map<string, string>();

  loadFtl(source: string): void {
    for (const rawLine of source.split(/\r?\n/)) {
      const line = rawLine.trim();
      if (!line || line.startsWith('#')) continue;
      const separator = line.indexOf('=');
      if (separator <= 0) continue;
      const key = line.slice(0, separator).trim();
      const value = line.slice(separator + 1).trim();
      this.messages.set(key, value);
    }
  }

  get(key: string, fallback = key): string {
    return this.messages.get(key) ?? fallback;
  }
}
