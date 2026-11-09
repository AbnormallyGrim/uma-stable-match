/** The announcement of the Applications and Recruitment Board pages, and who still gets it. */

/** Local storage key, set once the announcement is shown on this device. */
export const ANNOUNCEMENT_KEY = 'uma-stable-match:pages-announcement-seen';

/** Whether `pathname` is one of the announced pages, where the visitor needs no pointer to them. */
export function isAnnouncedPage(pathname: string): boolean {
  return /^\/(?:applications|recruitment-board)(?:\/|$)/.test(pathname);
}

/** Whether the announcement was shown on this device. Blocked storage counts as seen. */
export function hasSeenAnnouncement(storage: Storage | undefined): boolean {
  try {
    return storage === undefined || storage.getItem(ANNOUNCEMENT_KEY) !== null;
  } catch {
    return true;
  }
}

/** Remembers that the announcement was shown. Storage errors are ignored. */
export function markAnnouncementSeen(storage: Storage | undefined) {
  try {
    storage?.setItem(ANNOUNCEMENT_KEY, '1');
  } catch {
    return;
  }
}

/**
 * Whether a visit to `pathname` gets the announcement: not on the announced pages or the sign-in,
 * and not once seen. The search page shows it after its tour, which it waits for itself.
 */
export function shouldAnnounce(pathname: string, storage: Storage | undefined): boolean {
  return !isAnnouncedPage(pathname) && pathname !== '/login' && !hasSeenAnnouncement(storage);
}
