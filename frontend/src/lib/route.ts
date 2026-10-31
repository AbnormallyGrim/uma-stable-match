/** Which page an address shows: the search, or a club page at `/club/{id}?trainer=…`. */

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
