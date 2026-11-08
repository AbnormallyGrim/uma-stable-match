/** Remembering who is signed in between pages, so each page needn't ask the server. */

import type { Account } from './applications';

/** The cookie the server sets beside the session, holding its expiry. Nothing secret. */
export const SIGNED_IN_COOKIE = 'uma_signed_in';

/** Where the signed-in person is kept, with the session they belong to. */
export const ACCOUNT_KEY = 'uma-stable-match:account';

interface Saved {
  session: string;
  discordId: string;
  name: string | null;
}

/** The signed-in marker's value, or `null` when there's no session to count on. */
export function sessionMarker(cookies: string = document.cookie): string | null {
  for (const part of cookies.split(';')) {
    const [name, ...value] = part.trim().split('=');
    if (name === SIGNED_IN_COOKIE) {
      return value.join('=') || null;
    }
  }
  return null;
}

/** The person saved for the session `marker` names, or `null` if none is. */
export function savedAccount(marker: string, storage: Storage | undefined): Account | null {
  try {
    const saved = JSON.parse(storage?.getItem(ACCOUNT_KEY) ?? 'null') as Saved | null;
    if (saved?.session === marker && typeof saved.discordId === 'string') {
      return { discordId: saved.discordId, name: saved.name ?? null };
    }
  } catch {
    // Unreadable: ask the server again.
  }
  return null;
}

/** Keeps `account` for the session `marker` names. */
export function saveAccount(marker: string, account: Account, storage: Storage | undefined): void {
  const saved: Saved = { session: marker, discordId: account.discordId, name: account.name };
  try {
    storage?.setItem(ACCOUNT_KEY, JSON.stringify(saved));
  } catch {
    // Storage is full or blocked: the next page asks the server.
  }
}

/** Drops the saved person and the marker, so pages stop counting on a session. */
export function forgetSession(storage: Storage | undefined = localStorageOrNone()): void {
  try {
    storage?.removeItem(ACCOUNT_KEY);
  } catch {
    // Blocked storage holds nothing to forget.
  }
  if (typeof document !== 'undefined') {
    document.cookie = `${SIGNED_IN_COOKIE}=; Path=/; Max-Age=0; SameSite=Lax`;
  }
}

/** Local storage, or `undefined` where it isn't available. */
export function localStorageOrNone(): Storage | undefined {
  try {
    return typeof localStorage === 'undefined' ? undefined : localStorage;
  } catch {
    return undefined;
  }
}
