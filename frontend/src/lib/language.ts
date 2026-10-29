/** Language choice: saving it and working out which one to show. */

/** The languages the site is offered in. */
export type Language = 'en' | 'ja';

/** Local storage key for the chosen language. The website uses the same key. */
export const LANGUAGE_KEY = 'uma-stable-match:language';

/** Whether a value is a supported language. */
export function isLanguage(value: unknown): value is Language {
  return value === 'en' || value === 'ja';
}

/** The saved choice, or English if there's none or storage is blocked. */
export function loadLanguage(storage: Storage | undefined): Language {
  try {
    const value = storage?.getItem(LANGUAGE_KEY);
    return isLanguage(value) ? value : 'en';
  } catch {
    return 'en';
  }
}

/** Saves the choice. Storage errors are ignored. */
export function saveLanguage(storage: Storage | undefined, language: Language) {
  try {
    storage?.setItem(LANGUAGE_KEY, language);
  } catch {
    return;
  }
}

/** The language to show: the saved one if Japanese is enabled, otherwise English. */
export function resolveLanguage(japaneseEnabled: boolean, saved: Language): Language {
  return japaneseEnabled ? saved : 'en';
}
