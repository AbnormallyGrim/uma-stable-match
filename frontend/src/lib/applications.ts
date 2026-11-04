/** The club applications side of the API: Discord sign-in, opting a club in, and its settings. */

import { ApiError, type RequestOptions } from './api';
import { APPLICATIONS_PATH, settingsPath } from './route';

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

/** The address that starts opting a club in, ending on the club's settings page. */
export function optInUrl(clubId: string): string {
  return `/api/v1/discord/opt-in?${new URLSearchParams({ club: clubId, next: settingsPath(clubId, true) })}`;
}

/**
 * Parses a club ID taken.
 * Only raw numeric club IDs are accepted
 */
export function parseClubInput(input: string): string | null {
  const text = input.trim();
  return /^\d{1,18}$/.test(text) ? text : null;
}

/** The Discord ID of the signed-in leader, or `null` when nobody is signed in. */
export async function signedInUser(options: RequestOptions = {}): Promise<string | null> {
  try {
    const body = await send<{ discord_id: string }>('/api/v1/applications/me', {}, options);
    return body.discord_id;
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

/**
 * Checks that opting `clubId` in can start, without leaving the page: the server answers with a
 * redirect to Discord when the club has exactly one working invite, and with an error otherwise.
 *
 * Throws `ApiError`, with the server's status.
 */
export async function checkOptIn(clubId: string, options: RequestOptions = {}): Promise<void> {
  const fetchFn = options.fetch ?? globalThis.fetch;
  let response: Response;
  try {
    response = await fetchFn(optInUrl(clubId), { redirect: 'manual', signal: options.signal });
  } catch {
    throw new ApiError('network', 'Could not reach the server.');
  }
  // A manual redirect shows up as an opaque response, or with status 0 or a 3xx.
  if (response.type === 'opaqueredirect' || response.status === 0 || response.status < 400) {
    return;
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
