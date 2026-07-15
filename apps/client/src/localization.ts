type LocalizationManifest = {
  version: number;
  locales: Record<string, Record<string, string>>;
};

export class LocalizationBundle {
  private readonly messages = new Map<string, string>();
  private locale = 'en-US';

  public async initialize(preferredLocale?: string): Promise<void> {
    const response = await fetch('/Resources/Localization/manifest.json', {
      cache: 'no-cache',
    });
    if (!response.ok) {
      return;
    }
    const manifest = await response.json() as LocalizationManifest;
    const requested = preferredLocale ?? navigator.language;
    const available = Object.keys(manifest.locales);
    const locale =
      available.find((candidate) => candidate.toLowerCase() === requested.toLowerCase()) ??
      available.find((candidate) =>
        candidate.toLowerCase().startsWith(requested.split('-')[0]?.toLowerCase() ?? ''),
      ) ??
      available.find((candidate) => candidate === 'en-US') ??
      available[0];
    if (!locale) return;
    this.locale = locale;
    this.messages.clear();
    for (const [key, value] of Object.entries(manifest.locales[locale] ?? {})) {
      this.messages.set(key, value);
    }
  }

  public loadFtl(source: string): void {
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

  public get currentLocale(): string {
    return this.locale;
  }

  public get(key: string, fallback = key): string {
    return this.messages.get(key) ?? fallback;
  }
}
