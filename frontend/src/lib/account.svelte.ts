/** Who is signed in with Discord, shared by every component. */

import { type Account, signedInUser } from './applications';
import { flags } from './flags.svelte';

/** The signed-in person, `null` when nobody is; `known` once the server has answered. */
export const account = $state<{ current: Account | null; known: boolean }>({
  current: null,
  known: false,
});

let loading: Promise<Account | null> | null = null;

/** Asks the server who is signed in, once per page; `null` when nobody is or it can't be asked. */
export function loadAccount(): Promise<Account | null> {
  loading ??= signedInUser().then(
    (found) => {
      account.current = found;
      account.known = true;
      return found;
    },
    () => {
      account.known = true;
      return null;
    },
  );
  return loading;
}

/** Forgets the signed-in person, after signing out or deleting the account. */
export function clearAccount(): void {
  account.current = null;
  account.known = true;
  loading = Promise.resolve(null);
}

/** Whether the navbar holds the account menu or the sign-in button, which take room. */
export function showsAccount(): boolean {
  return account.current !== null || (account.known && flags.applications);
}
