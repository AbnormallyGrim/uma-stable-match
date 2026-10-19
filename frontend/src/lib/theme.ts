export type ThemeChoice = 'system' | 'light' | 'dark';

export const THEME_KEY = 'uma-stable-match:theme';

export function isThemeChoice(value: unknown): value is ThemeChoice {
  return value === 'system' || value === 'light' || value === 'dark';
}

export function loadTheme(storage: Storage | undefined): ThemeChoice {
  try {
    const value = storage?.getItem(THEME_KEY);
    return isThemeChoice(value) ? value : 'system';
  } catch {
    return 'system';
  }
}

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

export function applyTheme(choice: ThemeChoice, root: HTMLElement = document.documentElement) {
  if (choice === 'system') {
    delete root.dataset.theme;
  } else {
    root.dataset.theme = choice;
  }
}
