/** Looking up messages in the language being shown. */

import type { Language } from './language';
import { language } from './language.svelte';
import { en, type Catalog } from './messages/en';

/** The name of a message. */
export type MessageKey = keyof Catalog;

type Params<K extends MessageKey> = Catalog[K] extends (params: infer P) => string ? P : never;

// A language without a catalog yet shows English.
const catalogs: Partial<Record<Language, Catalog>> = { en };

/**
 * The message `key` in the language being shown, filled in with `params`.
 *
 * Reads the shown language, so a component or `$derived` that calls it updates on a switch.
 */
export function t<K extends MessageKey>(
  key: K,
  ...params: [Params<K>] extends [never] ? [] : [Params<K>]
): string {
  const message = (catalogs[language.current] ?? en)[key] as string | ((p: unknown) => string);
  return typeof message === 'function' ? message(params[0]) : message;
}

/** Whether `key` names a message. */
export function hasMessage(key: string): key is MessageKey {
  return Object.hasOwn(en, key);
}

/** The shown language as a locale for `Intl`. */
export function locale(): string {
  return language.current;
}
