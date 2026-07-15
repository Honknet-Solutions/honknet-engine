const IDENTITY_STORAGE_KEY = 'honknet.identityId';
const LEGACY_IDENTITY_STORAGE_KEY = 'honknet.guestIdentityId';
const AUTH_TOKEN_STORAGE_KEY = 'honknet.authToken';

export function getOrCreateGuestIdentityId(): string {
  const queryIdentity = new URLSearchParams(window.location.search)
    .get('identity')
    ?.trim();
  if (queryIdentity) {
    localStorage.setItem(IDENTITY_STORAGE_KEY, queryIdentity);
    return queryIdentity;
  }

  const existing =
    localStorage.getItem(IDENTITY_STORAGE_KEY)?.trim() ||
    localStorage.getItem(LEGACY_IDENTITY_STORAGE_KEY)?.trim();
  if (existing) {
    localStorage.setItem(IDENTITY_STORAGE_KEY, existing);
    localStorage.removeItem(LEGACY_IDENTITY_STORAGE_KEY);
    return existing;
  }

  const identity = `guest-${crypto.randomUUID()}`;
  localStorage.setItem(IDENTITY_STORAGE_KEY, identity);
  return identity;
}

export function getAuthToken(): string | null {
  const queryToken = new URLSearchParams(window.location.search)
    .get('token')
    ?.trim();
  if (queryToken) {
    localStorage.setItem(AUTH_TOKEN_STORAGE_KEY, queryToken);
    return queryToken;
  }

  return localStorage.getItem(AUTH_TOKEN_STORAGE_KEY)?.trim() || null;
}

export function setIdentityCredentials(identity: string, token: string): void {
  localStorage.setItem(IDENTITY_STORAGE_KEY, identity.trim());
  localStorage.setItem(AUTH_TOKEN_STORAGE_KEY, token.trim());
}

export function clearIdentityCredentials(): void {
  localStorage.removeItem(IDENTITY_STORAGE_KEY);
  localStorage.removeItem(LEGACY_IDENTITY_STORAGE_KEY);
  localStorage.removeItem(AUTH_TOKEN_STORAGE_KEY);
}
