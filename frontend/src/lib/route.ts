/**
 * Which page an address shows: the search, a club page at `/club/{id}?trainer=…`, the wishlist page,
 * the club applications pages, the settings page or the login page.
 */

import { parseRequirement, type Requirement } from './requirement';
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

/** The address of the club applications page, where a leader sets a club up. */
export const APPLICATIONS_PATH = '/applications';

/** The address of the page where a trainer follows the applications they've sent. */
export const TRAINER_APPLICATIONS_PATH = `${APPLICATIONS_PATH}/trainer`;

/** The address of the recruitment board, where trainers post and leaders look for trainers. */
export const BOARD_PATH = '/recruitment-board';

/** Where the board used to be: links to it are sent on to `BOARD_PATH`. */
export const LEGACY_BOARD_PATH = `${APPLICATIONS_PATH}/ads`;

/** The board's address for a visit to `pathname` and `search` at its old address, else `null`. */
export function legacyBoardRedirect(pathname: string, search: string): string | null {
  return pathname === LEGACY_BOARD_PATH || pathname === `${LEGACY_BOARD_PATH}/`
    ? `${BOARD_PATH}${search}`
    : null;
}

/** The address of the leader's page, which finds their club from their linked trainer. */
export const CLUB_APPLICATIONS_PATH = `${APPLICATIONS_PATH}/club`;

/**
 * A club applications page: the start, the leader's page (with the club when the address names
 * one), or a trainer's own.
 */
export type ApplicationsRoute =
  | { page: 'home'; error: string | null }
  | { page: 'club'; clubId: string | null; linked: boolean }
  | { page: 'compare'; clubId: string; trainerIds: string[]; requirement: Requirement | null }
  | { page: 'trainer' }
  | { page: 'board'; post: boolean };

const CLUB_APPLICATIONS = /^\/applications\/club(?:\/(\d{1,18}))?\/?$/;
const CLUB_COMPARE = /^\/applications\/club\/(\d{1,18})\/compare\/?$/;

/** Most trainers one comparison shows. */
export const MAX_COMPARED = 5;

/** The valid trainer IDs in `value`, comma separated, without repeats, at most `MAX_COMPARED`. */
function comparedTrainers(value: string | null): string[] {
  const ids = (value ?? '').split(',').filter(isTrainerId).map(normaliseTrainerId);
  return [...new Set(ids)].slice(0, MAX_COMPARED);
}

/** The club applications page `pathname` and `search` point to, or `null` for any other page. */
export function applicationsRoute(pathname: string, search: string): ApplicationsRoute | null {
  const params = new URLSearchParams(search);
  if (pathname === APPLICATIONS_PATH || pathname === `${APPLICATIONS_PATH}/`) {
    const error = params.get('error');
    // Keeps the error when it is a short label, as the page shows it.
    return { page: 'home', error: error !== null && /^[a-z_]{1,32}$/.test(error) ? error : null };
  }
  if (pathname === TRAINER_APPLICATIONS_PATH || pathname === `${TRAINER_APPLICATIONS_PATH}/`) {
    return { page: 'trainer' };
  }
  if (pathname === BOARD_PATH || pathname === `${BOARD_PATH}/`) {
    return { page: 'board', post: params.get('post') === '1' };
  }
  const compare = CLUB_COMPARE.exec(pathname);
  if (compare) {
    return {
      page: 'compare',
      clubId: compare[1]!,
      trainerIds: comparedTrainers(params.get('trainers')),
      requirement: parseRequirement(params.get('need'), params.get('per')),
    };
  }
  const match = CLUB_APPLICATIONS.exec(pathname);
  return match
    ? { page: 'club', clubId: match[1] ?? null, linked: params.get('linked') === '1' }
    : null;
}

/** A club's applications page, where its leaders add the bot and follow applications. */
export function clubApplicationsPath(clubId: number | string, linked = false): string {
  return `${APPLICATIONS_PATH}/club/${clubId}${linked ? '?linked=1' : ''}`;
}

/** The recruitment board, opening the trainer's own listing when `post` is set. */
export function boardPath(post = false): string {
  return post ? `${BOARD_PATH}?post=1` : BOARD_PATH;
}

/** The page where a leader compares applicants in their club, such as `…/compare?trainers=…`. */
export function clubComparePath(
  clubId: number | string,
  trainerIds: string[] = [],
  requirement: Requirement | null = null,
): string {
  const path = `${APPLICATIONS_PATH}/club/${clubId}/compare`;
  const params = new URLSearchParams();
  if (trainerIds.length > 0) {
    params.set('trainers', trainerIds.join(','));
  }
  if (requirement) {
    params.set('need', String(requirement.fans));
    params.set('per', requirement.per);
  }
  return params.size === 0 ? path : `${path}?${params}`;
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

/** The address of the settings page, where a signed-in person manages their account. */
export const SETTINGS_PATH = '/settings';

/** Whether `pathname` is the settings page. */
export function isSettingsRoute(pathname: string): boolean {
  return pathname === SETTINGS_PATH || pathname === `${SETTINGS_PATH}/`;
}

/** The address of the login page for Discord sign-in. */
export const LOGIN_PATH = '/login';

/** Whether `pathname` is the login page. */
export function isLoginRoute(pathname: string): boolean {
  return pathname === LOGIN_PATH || pathname === `${LOGIN_PATH}/`;
}
