/** The club applications side of the API: Discord sign-in, adding the bot and club settings. */

import { ApiError, type RequestOptions } from './api';
import { APPLICATIONS_PATH, clubApplicationsPath } from './route';

/** A category or role on the club's Discord server. */
export interface Named {
  id: string;
  name: string;
}

/** A club's Discord settings, with the server's categories and roles to choose from. */
export interface Settings {
  guild_id: string;
  category: string | null;
  leader_roles: string[];
  auto_join: boolean;
  open: boolean;
  problem: string | null;
  categories: Named[];
  roles: Named[];
}

/** Where an application stands with the club's leaders. */
export type ApplicationStatus = 'pending' | 'accepted' | 'declined';

/** An application to a club, as the bot posted it to the club's Discord server. */
export interface Application {
  id: string;
  trainerName: string;
  trainerId: string;
  /** The applicant's Discord username. */
  discordName: string;
  /** Fans per day when they applied. */
  pace: number;
  fansThisMonth: number;
  /** The club they were in when they applied, or `null` for none. */
  currentClub: string | null;
  status: ApplicationStatus;
  /** When they applied, as an ISO 8601 date and time. */
  submittedAt: string;
  /** The application's ticket channel in Discord, or `null` once it's gone. */
  ticketUrl: string | null;
}

/** What to change. Fields left out stay as they are; `category: null` clears the category. */
export interface SettingsChange {
  category?: string | null;
  leader_roles?: string[];
  auto_join?: boolean;
  open?: boolean;
}

/** The address that sends the visitor to sign in with Discord and back to `next`. */
export function loginUrl(next: string = APPLICATIONS_PATH): string {
  return `/api/v1/auth/discord/login?${new URLSearchParams({ next })}`;
}

/** The person signed in with Discord. */
export interface Account {
  discordId: string;
  /** Their Discord display name, or `null` when the server couldn't look it up. */
  name: string | null;
}

/** The signed-in person, or `null` when nobody is signed in. */
export async function signedInUser(options: RequestOptions = {}): Promise<Account | null> {
  try {
    const body = await send<{ discord_id: string; name?: string | null }>(
      '/api/v1/applications/me',
      {},
      options,
    );
    return { discordId: body.discord_id, name: body.name ?? null };
  } catch (error) {
    if (error instanceof ApiError && error.status === 401) {
      return null;
    }
    throw error;
  }
}

/** Ends the session. */
export async function signOut(options: RequestOptions = {}): Promise<void> {
  await send<null>('/api/v1/auth/sign-out', { method: 'POST' }, options);
}

/** Deletes the person's trainer links and pending keys, and ends the session. */
export async function deleteAccount(options: RequestOptions = {}): Promise<void> {
  await send<null>('/api/v1/auth/account', { method: 'DELETE' }, options);
}

/** A linked trainer: its ID, and its name when it linked, if kept. */
export interface LinkedTrainer {
  id: string;
  name: string | null;
}

/** Where the background check for a key stands. */
export type LinkCheck =
  { state: 'checking'; seconds_left: number } | { state: 'missing' | 'not_found' | 'too_many' };

/** A key waiting to be found in a trainer's profile, and its check if one ran lately. */
export interface PendingLink {
  trainer: string;
  key: string;
  expires_in: number;
  check: LinkCheck | null;
}

/** The signed-in person's linked trainers, how many they can link, and their pending keys. */
export interface LinkedTrainers {
  linked: LinkedTrainer[];
  max: number;
  /** Newest first. */
  pending: PendingLink[];
}

/** A key to put in the trainer's profile comment, and the seconds it has left. */
export interface VerificationKey {
  key: string;
  expires_in: number;
}

/** The signed-in person's linked trainer IDs. */
export async function fetchLinkedTrainers(options: RequestOptions = {}): Promise<LinkedTrainers> {
  return send<LinkedTrainers>('/api/v1/discord/trainers', {}, options);
}

/** Starts linking `trainerId`: the key to put in its profile comment. Throws 409 if it's linked. */
export async function requestVerificationKey(
  trainerId: string,
  options: RequestOptions = {},
): Promise<VerificationKey> {
  return send<VerificationKey>(
    `${trainerUrl(trainerId)}/verification`,
    { method: 'POST' },
    options,
  );
}

/** What pressing Check did: linked at once, or a check that keeps going on the server. */
export type CheckResult = { linked: true; name: string } | { checking: true; seconds_left: number };

/**
 * Looks for the key in `trainerId`'s profile comment, linking it if it's there; otherwise the
 * server keeps looking every minute for 5 minutes. Throws 409 without a key, 404 for an unknown
 * trainer, 403 with no room, 429 for too many checks.
 */
export async function checkTrainerLink(
  trainerId: string,
  options: RequestOptions = {},
): Promise<CheckResult> {
  return send<CheckResult>(`${trainerUrl(trainerId)}/link`, { method: 'POST' }, options);
}

/** Forgets the key pending for `trainerId`, stopping any check for it. */
export async function discardVerificationKey(
  trainerId: string,
  options: RequestOptions = {},
): Promise<void> {
  await send<null>(`${trainerUrl(trainerId)}/verification`, { method: 'DELETE' }, options);
}

/** Unlinks `trainerId` from the signed-in person. */
export async function unlinkTrainer(
  trainerId: string,
  options: RequestOptions = {},
): Promise<void> {
  await send<null>(trainerUrl(trainerId), { method: 'DELETE' }, options);
}

function trainerUrl(trainerId: string): string {
  return `/api/v1/discord/trainers/${encodeURIComponent(trainerId)}`;
}

/** The address that adds the bot to the club's server, coming back to the club's page. */
export function optInUrl(clubId: string): string {
  return `/api/v1/discord/opt-in?${new URLSearchParams({ club: clubId, next: clubApplicationsPath(clubId, true) })}`;
}

/**
 * Whether the club's description has a working invite to one Discord server, without leaving the
 * page: the opt-in address only redirects to Discord when it does. Throws `ApiError` for anything
 * else, such as 401 when signed out.
 */
export async function hasInvite(clubId: string, options: RequestOptions = {}): Promise<boolean> {
  const fetchFn = options.fetch ?? globalThis.fetch;
  let response: Response;
  try {
    response = await fetchFn(optInUrl(clubId), {
      credentials: 'same-origin',
      redirect: 'manual',
      signal: options.signal,
    });
  } catch (error) {
    if (options.signal?.aborted) {
      throw error;
    }
    throw new ApiError('network', 'Could not reach the server.');
  }
  if (response.type === 'opaqueredirect' || (response.status >= 300 && response.status < 400)) {
    return true;
  }
  if (response.status === 422) {
    return false;
  }
  throw await httpError(response);
}

/** The club's settings. Throws `ApiError`: 401 signed out, 403 not allowed, 404 not linked. */
export async function fetchSettings(
  clubId: string,
  options: RequestOptions = {},
): Promise<Settings> {
  return send<Settings>(settingsUrl(clubId), {}, options);
}

/** Changes the club's settings, giving the new ones. */
export async function saveSettings(
  clubId: string,
  change: SettingsChange,
  options: RequestOptions = {},
): Promise<Settings> {
  return send<Settings>(
    settingsUrl(clubId),
    {
      method: 'PATCH',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(change),
    },
    options,
  );
}

/** Unlinks the club from its Discord server. */
export async function unlinkClub(clubId: string, options: RequestOptions = {}): Promise<void> {
  await send<null>(
    `/api/v1/discord/clubs/${encodeURIComponent(clubId)}`,
    { method: 'DELETE' },
    options,
  );
}

function settingsUrl(clubId: string): string {
  return `/api/v1/discord/clubs/${encodeURIComponent(clubId)}/settings`;
}

async function send<T>(path: string, init: RequestInit, options: RequestOptions): Promise<T> {
  const fetchFn = options.fetch ?? globalThis.fetch;
  let response: Response;
  try {
    response = await fetchFn(path, {
      ...init,
      headers: { Accept: 'application/json', ...init.headers },
      credentials: 'same-origin',
      signal: options.signal,
    });
  } catch (error) {
    if (options.signal?.aborted) {
      throw error;
    }
    throw new ApiError('network', 'Could not reach the server.');
  }
  if (!response.ok) {
    throw await httpError(response);
  }
  if (response.status === 204) {
    return null as T;
  }
  try {
    return (await response.json()) as T;
  } catch {
    throw new ApiError('invalid-response', 'The server sent an unexpected response.');
  }
}

async function httpError(response: Response): Promise<ApiError> {
  const body: unknown = await response.json().catch(() => null);
  const message =
    body !== null && typeof body === 'object' && 'error' in body && typeof body.error === 'string'
      ? body.error
      : `Request failed with status ${response.status}.`;
  return new ApiError('http', message, response.status);
}
