/** The board of ads: trainers post that they're looking for a club, and leaders invite them. */

import type { RequestOptions } from './api';
import { send } from './applications';
import { formatDate, TIER_LABELS } from './format';
import { t } from './i18n';

/** Whether a trainer wants the club to have a Discord server. */
export type DiscordWish = 'required' | 'preferred' | 'any';

/** The languages an ad can ask for, by two-letter code, and `any`. Only filters, not translations. */
export const LANGUAGE_CODES = [
  'en',
  'ja',
  'ko',
  'zh',
  'es',
  'pt',
  'fr',
  'de',
  'id',
  'th',
  'vi',
  'any',
] as const;

/** The language a trainer wants the club to speak. */
export type AdLanguage = (typeof LANGUAGE_CODES)[number];

/** The club ranks an ad can ask for, as tier codes from 1 (D) to 11 (SS). */
export const RANK_CODES = TIER_LABELS.map((_, index) => index + 1);

/** Most club ranks one ad can ask for, both ends counted: C to B (C, C+ and B) is three. */
export const MAX_RANK_SPAN = 3;

/** The tier a rank code stands for, such as `"A+"`. */
export function rankLabel(code: number): string {
  return TIER_LABELS[code - 1] ?? String(code);
}

/** What a trainer chooses when they post an ad. */
export interface AdChoices {
  rankMin: number;
  rankMax: number;
  discord: DiscordWish;
  language: AdLanguage;
}

/** What a trainer starts with when they have no ad: the three lowest club ranks. */
export const DEFAULT_CHOICES: AdChoices = {
  rankMin: 1,
  rankMax: MAX_RANK_SPAN,
  discord: 'any',
  language: 'any',
};

/** A club's invite to the trainer, with the club's name when it could be looked up. */
export interface Invite {
  clubId: string;
  clubName: string | null;
  /** When it was sent, as an ISO 8601 date and time. */
  sentAt: string;
}

/** The signed-in trainer's own ad. */
export interface MyAd extends AdChoices {
  trainerName: string;
  trainerId: string;
  currentClub: string | null;
  /** Fans per day, or `null` when there isn't enough data. */
  pace: number | null;
  fansThisMonth: number | null;
  /** When it was first posted. */
  postedAt: string;
  /** When it leaves the board unless renewed. */
  visibleUntil: string;
  /** When it's deleted unless renewed. */
  expiresAt: string;
  onBoard: boolean;
  /** Newest first. */
  invites: Invite[];
}

/** An ad on the board, as a leader sees it. */
export interface BoardAd {
  id: string;
  trainerName: string;
  trainerId: string;
  currentClub: string | null;
  pace: number | null;
  fansThisMonth: number | null;
  rankMin: number;
  rankMax: number;
  discord: DiscordWish;
  language: AdLanguage;
  postedAt: string;
  visibleUntil: string;
  /** Whether the leader's club already invited them. */
  invited: boolean;
}

const JSON_HEADERS = { 'Content-Type': 'application/json' };

/** The signed-in trainer's ad, or `null` without one. */
export async function fetchMyAd(options: RequestOptions = {}): Promise<MyAd | null> {
  return send<MyAd | null>('/api/v1/discord/ads/me', {}, options);
}

/** Posts the ad, or replaces the earlier one, putting it on the board for another week. */
export async function saveAd(choices: AdChoices, options: RequestOptions = {}): Promise<MyAd> {
  return send<MyAd>(
    '/api/v1/discord/ads/me',
    { method: 'PUT', headers: JSON_HEADERS, body: JSON.stringify(choices) },
    options,
  );
}

/** Puts the ad back on the board for another week, with fresh fan numbers. */
export async function renewAd(options: RequestOptions = {}): Promise<MyAd> {
  return send<MyAd>('/api/v1/discord/ads/me/renew', { method: 'POST' }, options);
}

/** Deletes the ad and its invites. */
export async function deleteAd(options: RequestOptions = {}): Promise<void> {
  await send<null>('/api/v1/discord/ads/me', { method: 'DELETE' }, options);
}

/** Dismisses a club's invite. */
export async function dismissInvite(clubId: string, options: RequestOptions = {}): Promise<void> {
  await send<null>(
    `/api/v1/discord/ads/me/invites/${encodeURIComponent(clubId)}`,
    { method: 'DELETE' },
    options,
  );
}

/** One page of the board, as the server filtered and paged it. */
export interface BoardPage {
  /** Fastest first. */
  ads: BoardAd[];
  /** The page shown, from 1. */
  page: number;
  /** How many pages there are, at least one. */
  pages: number;
  /** How many ads match the filters. */
  total: number;
}

/**
 * One page of the ads that match `filters`. With `clubId`, the club the signed-in person leads,
 * each ad also says whether that club invited its trainer; it throws `ApiError` 403 for anyone
 * else.
 */
export async function fetchBoard(
  clubId: string | null,
  filters: BoardFilters,
  page: number,
  options: RequestOptions = {},
): Promise<BoardPage> {
  // Only the filters that are set go to the server; the rest filter nothing.
  const query = new URLSearchParams({ page: String(page) });
  if (filters.rankMin !== null) {
    query.set('rank_min', String(filters.rankMin));
  }
  if (filters.rankMax !== null) {
    query.set('rank_max', String(filters.rankMax));
  }
  if (filters.hasDiscord !== null) {
    query.set('has_discord', String(filters.hasDiscord));
  }
  if (filters.language !== 'any') {
    query.set('language', filters.language);
  }
  if (clubId !== null) {
    query.set('club', clubId);
  }
  return send<BoardPage>(`/api/v1/discord/ads?${query}`, {}, options);
}

/** Invites the trainer behind an ad to apply to club `clubId`. */
export async function inviteTrainer(
  clubId: string,
  adId: string,
  options: RequestOptions = {},
): Promise<void> {
  await send<null>(
    `/api/v1/discord/clubs/${encodeURIComponent(clubId)}/ads/${encodeURIComponent(adId)}/invite`,
    { method: 'POST' },
    options,
  );
}

/** How a leader narrows the board down, by what their club is like. The server applies them. */
export interface BoardFilters {
  /** The ad's lowest rank must be at or above this tier code; `null` is unset. */
  rankMin: number | null;
  /** The ad's highest rank must be at or below this tier code; `null` is unset. */
  rankMax: number | null;
  /** Whether the club has a Discord server: without one, trainers who require one are left out. */
  hasDiscord: boolean | null;
  /** The language the club speaks: ads for another language are left out; `any` is unset. */
  language: AdLanguage;
}

/** Every filter unset, which shows every ad. */
export const NO_FILTERS: BoardFilters = {
  rankMin: null,
  rankMax: null,
  hasDiscord: null,
  language: 'any',
};

/** The address of a trainer's uma.moe profile. */
export function profileUrl(trainerId: string): string {
  return `https://uma.moe/profile/${encodeURIComponent(trainerId)}`;
}

/** A club rank range for showing: `Any rank`, one tier, or `C to A+`. */
export function rankRange(min: number, max: number): string {
  if (min === RANK_CODES[0] && max === RANK_CODES[RANK_CODES.length - 1]) {
    return t('ads.table.anyRank');
  }
  return min === max
    ? rankLabel(min)
    : t('ads.table.ranks', { min: rankLabel(min), max: rankLabel(max) });
}

/** The date of an ISO 8601 timestamp, as the site shows dates. */
export function adDay(iso: string): string {
  return formatDate(new Date(iso).toISOString().slice(0, 10));
}
