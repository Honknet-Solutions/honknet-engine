const GUEST_IDENTITY_STORAGE_KEY =
  'ss15.guestIdentityId';

export function getOrCreateGuestIdentityId(): string {
  const existingIdentityId = localStorage.getItem(
    GUEST_IDENTITY_STORAGE_KEY,
  );

  if (
    existingIdentityId &&
    existingIdentityId.trim().length > 0
  ) {
    return existingIdentityId;
  }

  const newIdentityId = `guest-${crypto.randomUUID()}`;

  localStorage.setItem(
    GUEST_IDENTITY_STORAGE_KEY,
    newIdentityId,
  );

  return newIdentityId;
}