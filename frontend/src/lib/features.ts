/** Switching on what the server reports, once per browser session. */

import { loadWebAnalytics } from './analytics';
import { fetchFeatures, type Features, type RequestOptions } from './api';
import { language } from './language.svelte';

const STORAGE_KEY = 'uma-stable-match:features';

const sessionStore = (): Storage | null =>
  typeof sessionStorage === 'undefined' ? null : sessionStorage;

/**
 * Offers Japanese and loads the analytics beacon if the server has them on. Asks the server once
 * per session and reuses the answer. A failed request leaves them all off and asks again next time.
 */
export function applyFeatures(
  options: RequestOptions = {},
  storage: Storage | null = sessionStore(),
): Promise<void> {
  const saved = savedFeatures(storage);
  if (saved) {
    switchOn(saved);
    return Promise.resolve();
  }
  return fetchFeatures(options).then(
    (found) => {
      saveFeatures(storage, found);
      switchOn(found);
    },
    () => {},
  );
}

function switchOn(found: Features): void {
  language.japaneseEnabled = found.japanese;
  if (found.webAnalytics !== null) {
    loadWebAnalytics(found.webAnalytics);
  }
}

function savedFeatures(storage: Storage | null): Features | null {
  try {
    const raw = storage?.getItem(STORAGE_KEY);
    if (!raw) return null;
    const parsed = JSON.parse(raw) as Record<string, unknown>;
    if (typeof parsed.japanese !== 'boolean') return null;
    const token = parsed.webAnalytics;
    if (token !== null && (typeof token !== 'string' || token === '')) return null;
    return { japanese: parsed.japanese, webAnalytics: token };
  } catch {
    return null;
  }
}

function saveFeatures(storage: Storage | null, found: Features): void {
  try {
    storage?.setItem(STORAGE_KEY, JSON.stringify(found));
  } catch {
    // Without storage, the next page asks again.
  }
}
