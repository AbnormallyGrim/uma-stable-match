/** Which page an address shows: the search, a club page at `/club/{id}?trainer=…`, or the wishlist page. */

import { isTrainerId, normaliseTrainerId } from './search';

/** A club page address: the club, and the trainer to place among its members, if any. */
export interface ClubRoute {
  circleId: string;
  trainerId: string | null;
}

const CLUB_PATH = /^\/club\/(\d{1,18})\/?$/;

/** The club page `pathname` and `search` point to, or `null` for any other page. */
export function clubRoute(pathname: string, search: string): ClubRoute | null {
  const match = CLUB_PATH.exec(pathname);
  if (!match) {
    return null;
  }
  const trainer = new URLSearchParams(search).get('trainer');
  return {
    circleId: match[1]!,
    // A broken trainer ID is dropped, so the club still shows.
    trainerId: trainer !== null && isTrainerId(trainer) ? normaliseTrainerId(trainer) : null,
  };
}

/** The club page's address, such as `/club/123?trainer=123456789012`. */
export function clubPath(circleId: number | string, trainerId: string | null = null): string {
  const path = `/club/${circleId}`;
  return trainerId === null ? path : `${path}?${new URLSearchParams({ trainer: trainerId })}`;
}

/** The address of the wishlist page, where visitors vote on ideas. */
export const WISHLIST_PATH = '/wishlist';

/** Whether `pathname` is the wishlist page. */
export function isWishlistRoute(pathname: string): boolean {
  return pathname === WISHLIST_PATH || pathname === `${WISHLIST_PATH}/`;
}

const FROM_KEY = 'uma-stable-match:club-from';

/** Notes the search page's address before opening a club page, for its "Back to search". */
export function rememberSearch(storage: Storage | undefined, href: string): void {
  try {
    storage?.setItem(FROM_KEY, href);
  } catch {
    // Without storage, "Back to search" opens a new search.
  }
}

/** The search page's address noted by `rememberSearch`, if any. */
export function rememberedSearch(storage: Storage | undefined): string | null {
  try {
    return storage?.getItem(FROM_KEY) ?? null;
  } catch {
    return null;
  }
}
