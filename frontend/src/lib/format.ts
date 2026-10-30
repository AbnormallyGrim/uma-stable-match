/** Formatting and labels for showing results. */

import type { JoinStyle } from './api';
import { hasMessage, locale, t } from './i18n';

/** Fans in millions: one decimal from 1M (`6.4M`), two below (`0.85M`), `<0.01M` for tiny values. */
export function formatMillions(fans: number): string {
  const millions = fans / 1_000_000;
  if (millions === 0) {
    return '0M';
  }
  if (Math.abs(millions) < 0.01) {
    return '<0.01M';
  }
  const digits = Math.abs(millions) < 1 ? 2 : 1;
  return `${millions.toFixed(digits)}M`;
}

/** A daily gain, such as `6.4M/day`. */
export function formatPerDay(fans: number): string {
  return t('format.perDay', { value: formatMillions(fans) });
}

/** A daily pace: thousands below 1M (`850K/day`), plain below 1K, otherwise as `formatPerDay`. */
export function formatPace(fans: number): string {
  if (Math.abs(fans) >= 1_000_000) {
    return formatPerDay(fans);
  }
  const value = Math.abs(fans) < 1000 ? `${Math.round(fans)}` : `${Math.floor(fans / 1000)}K`;
  return t('format.perDay', { value });
}

/** A place in a club, such as `8 of 30`. */
export function formatPlace(place: number, size: number): string {
  return t('format.place', { place, size });
}

/** A share from 0 to 1 as a whole percentage, such as `34%`. */
export function formatPercent(share: number): string {
  return `${Math.round(share * 100)}%`;
}

/** How far a pace is above (`+3.6M/day`) or below (`−1.2M/day`) another. */
export function formatGap(fans: number): string {
  const sign = fans > 0 ? '+' : fans < 0 ? '−' : '';
  return `${sign}${formatPace(Math.abs(fans))}`;
}

/** Direction of a club's rank since last month. */
export type TrendKind = 'up' | 'down' | 'same' | 'new' | 'unknown';

/** A rank change: a short `label` to show and a `description` for screen readers. */
export interface RankTrend {
  kind: TrendKind;
  change: number;
  label: string;
  description: string;
}

/** How a club's monthly rank changed since last month. A lower rank number is better. */
export function rankTrend(rank: number | null, lastMonthRank: number | null): RankTrend {
  if (rank === null) {
    return { kind: 'unknown', change: 0, label: '', description: t('trend.unknown') };
  }
  if (lastMonthRank === null) {
    return { kind: 'new', change: 0, label: t('trend.new'), description: t('trend.notRanked') };
  }
  const change = lastMonthRank - rank;
  if (change > 0) {
    return {
      kind: 'up',
      change,
      label: `▲${change}`,
      description: t('trend.up', { n: change }),
    };
  }
  if (change < 0) {
    return {
      kind: 'down',
      change,
      label: `▼${-change}`,
      description: t('trend.down', { n: -change }),
    };
  }
  return { kind: 'same', change: 0, label: '–', description: t('trend.same') };
}

/** Club tiers from lowest to highest. */
export const TIER_LABELS = ['D', 'D+', 'C', 'C+', 'B', 'B+', 'A', 'A+', 'S', 'S+', 'SS'];

/** A tier's position from 0 (D) to 10 (SS), or -1 if unknown. */
export function tierIndex(tier: string | null): number {
  return tier === null ? -1 : TIER_LABELS.indexOf(tier);
}

/** Display name for a join style. */
export function joinStyleLabel(style: JoinStyle | null): string | null {
  return style === null ? null : t(`join.${style}`);
}

/** uma.moe's club policy codes, which the game shows as playstyles. */
export const PLAYSTYLE_CODES = Array.from({ length: 17 }, (_, i) => i + 1);

/** Every playstyle code with its name, in code order. */
export function playstyles(): [number, string][] {
  return PLAYSTYLE_CODES.map((code) => [code, playstyleLabel(code) ?? '']);
}

/** A club's playstyle name from its code, or `null` if unknown. */
export function playstyleLabel(policy: number | null): string | null {
  const key = `playstyle.${policy}`;
  return policy !== null && hasMessage(key) ? t(key) : null;
}

/** How many spots are open, such as `3 open spots`, or `Full`. `null` if unknown. */
export function spotsLabel(openSpots: number | null): string | null {
  if (openSpots === null) {
    return null;
  }
  return openSpots === 0 ? t('club.full') : t('club.openSpots', { n: openSpots });
}

const UNITS: [Intl.RelativeTimeFormatUnit, number][] = [
  ['day', 86_400],
  ['hour', 3_600],
  ['minute', 60],
];

/** How long ago a time was, such as `3 hours ago` or `yesterday`. Future times count as `just now`. */
export function formatUpdated(iso: string, now: Date = new Date()): string {
  const then = new Date(iso);
  if (Number.isNaN(then.getTime())) {
    return t('time.unknown');
  }
  const seconds = Math.round((now.getTime() - then.getTime()) / 1000);
  if (seconds < 60) {
    return t('time.justNow');
  }
  const format = new Intl.RelativeTimeFormat(locale(), { numeric: 'auto' });
  for (const [unit, size] of UNITS) {
    if (seconds >= size) {
      return format.format(-Math.floor(seconds / size), unit);
    }
  }
  return t('time.justNow');
}

const JST_OFFSET_MS = 9 * 3_600_000;

/** The game month containing a time, as `YYYY-MM`. Game months start on the 2nd in Japan time. */
export function gameMonth(iso: string): string | null {
  const time = new Date(iso).getTime();
  if (Number.isNaN(time)) {
    return null;
  }
  const jst = new Date(time + JST_OFFSET_MS);
  let year = jst.getUTCFullYear();
  let month = jst.getUTCMonth() + 1;
  if (jst.getUTCDate() < 2) {
    month -= 1;
    if (month === 0) {
      month = 12;
      year -= 1;
    }
  }
  return `${year}-${String(month).padStart(2, '0')}`;
}

/** Whole days from the start of a time's game month (the 2nd, 00:00 Japan time) to the time. */
export function daysIntoGameMonth(iso: string): number | null {
  const time = new Date(iso).getTime();
  const month = gameMonth(iso);
  if (month === null) {
    return null;
  }
  const [year, number] = month.split('-').map(Number);
  const start = Date.UTC(year!, number! - 1, 2) - JST_OFFSET_MS;
  return Math.floor((time - start) / 86_400_000);
}

/** `2026-02` → `February 2026`. Anything else is returned unchanged. */
export function monthLabel(month: string): string {
  const match = /^(\d{4})-(\d{2})$/.exec(month);
  const number = match ? Number(match[2]) : 0;
  return match && number >= 1 && number <= 12
    ? t('month.label', { year: match[1]!, month: number })
    : month;
}

/** `2026-10-24` → `October 24, 2026`, in the language shown. Anything else is returned unchanged. */
export function formatDate(date: string): string {
  const time = /^\d{4}-\d{2}-\d{2}$/.test(date) ? Date.parse(date) : Number.NaN;
  return Number.isNaN(time)
    ? date
    : new Intl.DateTimeFormat(locale(), { dateStyle: 'long', timeZone: 'UTC' }).format(time);
}
