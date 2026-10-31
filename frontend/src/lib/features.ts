/** Switching on what the server reports, on every page. */

import { loadWebAnalytics } from './analytics';
import { fetchFeatures, type RequestOptions } from './api';
import { language } from './language.svelte';

/**
 * Asks the server which features are on, then offers Japanese and loads the analytics beacon if
 * they are. A failed request leaves both off.
 */
export function applyFeatures(options: RequestOptions = {}): Promise<void> {
  return fetchFeatures(options).then(
    (found) => {
      language.japaneseEnabled = found.japanese;
      if (found.webAnalytics !== null) {
        loadWebAnalytics(found.webAnalytics);
      }
    },
    () => {},
  );
}
