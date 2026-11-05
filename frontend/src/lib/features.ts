/** Switching on what the server reports, on every page. */

import { loadWebAnalytics } from './analytics';
import { fetchFeatures, type RequestOptions } from './api';
import { flags } from './flags.svelte';
import { language } from './language.svelte';

/**
 * Asks the server which features are on, then offers Japanese, shows the applications link and
 * loads the analytics beacon if they are. A failed request leaves them all off.
 */
export function applyFeatures(options: RequestOptions = {}): Promise<void> {
  return fetchFeatures(options).then(
    (found) => {
      language.japaneseEnabled = found.japanese;
      flags.applications = found.applications;
      if (found.webAnalytics !== null) {
        loadWebAnalytics(found.webAnalytics);
      }
    },
    () => {},
  );
}
