/** Theme choice: saving it and applying it to the page. */

/** Follow the system setting, or force light or dark. */
export type ThemeChoice = 'system' | 'light' | 'dark';

/** Local storage key for a forced theme. Also read by the early script in `index.html`. */
export const THEME_KEY = 'uma-stable-match:theme';

/** Whether a value is a valid theme choice. */
export function isThemeChoice(value: unknown): value is ThemeChoice {
  return value === 'system' || value === 'light' || value === 'dark';
}

/** The saved choice, or `system` if there's none or storage is blocked. */
export function loadTheme(storage: Storage | undefined): ThemeChoice {
  try {
    const value = storage?.getItem(THEME_KEY);
    return isThemeChoice(value) ? value : 'system';
  } catch {
    return 'system';
  }
}

/** Saves a forced theme, or forgets it for `system`. Storage errors are ignored. */
export function saveTheme(storage: Storage | undefined, choice: ThemeChoice) {
  try {
    if (choice === 'system') {
      storage?.removeItem(THEME_KEY);
    } else {
      storage?.setItem(THEME_KEY, choice);
    }
  } catch {
    return;
  }
}

/** Sets `data-theme` on the root element, or removes it to follow the system. */
export function applyTheme(choice: ThemeChoice, root: HTMLElement = document.documentElement) {
  if (choice === 'system') {
    delete root.dataset.theme;
  } else {
    root.dataset.theme = choice;
  }
}
