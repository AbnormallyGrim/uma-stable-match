/** Who is signed in with Discord, shared by every component. */

import { type Account, signedInUser } from './applications';
import {
  ACCOUNT_KEY,
  forgetSession,
  localStorageOrNone,
  saveAccount,
  savedAccount,
  sessionMarker,
} from './session';

/** The signed-in person, `null` when nobody is; `known` once it has been worked out. */
export const account = $state<{ current: Account | null; known: boolean }>({
  current: null,
  known: false,
});

let loading: Promise<Account | null> | null = null;

/**
 * Who is signed in, once per page; `null` when nobody is or the server can't be asked. Without the
 * signed-in marker nobody is, and a person saved for the same session is reused, so the server is
 * only asked after signing in or when the saved person is missing.
 */
export function loadAccount(): Promise<Account | null> {
  loading ??= find();
  return loading;
}

async function find(): Promise<Account | null> {
  const storage = localStorageOrNone();
  const marker = sessionMarker();
  if (marker === null) {
    forgetSession(storage);
    return settle(null);
  }
  const saved = savedAccount(marker, storage);
  if (saved !== null) {
    return settle(saved);
  }
  let found: Account | null;
  try {
    found = await signedInUser();
  } catch {
    account.known = true;
    return null;
  }
  if (found === null) {
    forgetSession(storage);
  } else {
    saveAccount(marker, found, storage);
  }
  return settle(found);
}

function settle(found: Account | null): Account | null {
  account.current = found;
  account.known = true;
  return found;
}

/** Forgets the signed-in person, after signing out or deleting the account. */
export function clearAccount(): void {
  forgetSession();
  account.current = null;
  account.known = true;
  loading = Promise.resolve(null);
}

/** Whether the navbar holds the account menu or the sign-in button, which take room. */
export function showsAccount(): boolean {
  return account.current !== null || account.known;
}

// Signing out in another tab signs this one out too.
if (typeof window !== 'undefined') {
  window.addEventListener('storage', (event) => {
    if (event.key === ACCOUNT_KEY && event.newValue === null && account.current !== null) {
      account.current = null;
      account.known = true;
      loading = Promise.resolve(null);
    }
  });
}
