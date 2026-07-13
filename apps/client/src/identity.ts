const STORAGE_KEY = 'honknet.guestIdentityId';

export function getOrCreateGuestIdentityId(): string {
  const existing = localStorage.getItem(STORAGE_KEY)?.trim();
  if (existing) {
    return existing;
  }

  const identity = `guest-${crypto.randomUUID()}`;
  localStorage.setItem(STORAGE_KEY, identity);
  return identity;
}
