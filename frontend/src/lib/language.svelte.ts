/** The language shown right now, shared by every component. */

import {
  loadLanguage,
  resolveLanguage,
  saveLanguage,
  type CookieDocument,
  type Language,
} from './language';

/** Holds the saved choice and whether Japanese is enabled. */
export class LanguageState {
  /** Whether the server has Japanese switched on. Off until it says so. */
  japaneseEnabled = $state(false);
  private saved = $state<Language>('en');
  private readonly storage: Storage | undefined;
  private readonly cookies: CookieDocument | undefined;

  constructor(storage: Storage | undefined, cookies?: CookieDocument) {
    this.storage = storage;
    this.cookies = cookies;
    this.saved = loadLanguage(storage, cookies);
  }

  /** The language to show. English whenever Japanese is off, whatever was saved. */
  get current(): Language {
    return resolveLanguage(this.japaneseEnabled, this.saved);
  }

  /** Switches language and remembers it. */
  choose(language: Language) {
    this.saved = language;
    saveLanguage(this.storage, language, this.cookies);
  }
}

/** The app's language. */
export const language = new LanguageState(
  typeof localStorage === 'undefined' ? undefined : localStorage,
  typeof document === 'undefined' ? undefined : document,
);
