/** Formatting and labels for showing results. */

import type { JoinStyle } from './api';

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
  return `${formatMillions(fans)}/day`;
}

/** A place in a club, such as `8 of 30`. */
export function formatPlace(place: number, size: number): string {
  return `${place} of ${size}`;
}

/** A share from 0 to 1 as a whole percentage, such as `34%`. */
export function formatPercent(share: number): string {
  return `${Math.round(share * 100)}%`;
}

/** A fit or score to three decimals. */
export function formatScore(score: number): string {
  return score.toFixed(3);
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
    return { kind: 'unknown', change: 0, label: '', description: 'Rank unknown' };
  }
  if (lastMonthRank === null) {
    return { kind: 'new', change: 0, label: 'new', description: 'Not ranked last month' };
  }
  const change = lastMonthRank - rank;
  if (change > 0) {
    return {
      kind: 'up',
      change,
      label: `▲${change}`,
      description: `Up ${change} ${change === 1 ? 'place' : 'places'} since last month`,
    };
  }
  if (change < 0) {
    return {
      kind: 'down',
      change,
      label: `▼${-change}`,
      description: `Down ${-change} ${change === -1 ? 'place' : 'places'} since last month`,
    };
  }
  return { kind: 'same', change: 0, label: '–', description: 'Same rank as last month' };
}

/** Club tiers from lowest to highest. */
export const TIER_LABELS = ['D', 'D+', 'C', 'C+', 'B', 'B+', 'A', 'A+', 'S', 'S+', 'SS'];

/** A tier's position from 0 (D) to 10 (SS), or -1 if unknown. */
export function tierIndex(tier: string | null): number {
  return tier === null ? -1 : TIER_LABELS.indexOf(tier);
}

const JOIN_STYLE_LABELS: Record<JoinStyle, string> = {
  open: 'Open',
  approval: 'Approval',
  closed: 'Closed',
};

/** Display name for a join style. */
export function joinStyleLabel(style: JoinStyle | null): string | null {
  return style === null ? null : JOIN_STYLE_LABELS[style];
}

// uma.moe's names for club policy codes.
const PLAYSTYLE_LABELS: Record<number, string> = {
  1: 'You Do You',
  2: 'Laid-back',
  3: 'Going for Gold',
  4: 'Beginners Welcome',
  5: "Let's Party!",
  6: 'Rank 2000+',
  7: 'Rank 1000+',
  8: 'Rank 500+',
  9: 'Rank 250+',
  10: 'Rank 100+',
  11: 'Rank 20+',
  12: 'Log in Daily',
  13: 'Log in Every 3 Days',
  14: 'Active in the Morning',
  15: 'Active in the Afternoon',
  16: 'Active in the Evening',
  17: 'Active at Night',
};

/** uma.moe's name for a club's playstyle code, or `null` if unknown. */
export function playstyleLabel(policy: number | null): string | null {
  return policy === null ? null : (PLAYSTYLE_LABELS[policy] ?? null);
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
    return 'unknown';
  }
  const seconds = Math.round((now.getTime() - then.getTime()) / 1000);
  if (seconds < 60) {
    return 'just now';
  }
  const format = new Intl.RelativeTimeFormat('en', { numeric: 'auto' });
  for (const [unit, size] of UNITS) {
    if (seconds >= size) {
      return format.format(-Math.floor(seconds / size), unit);
    }
  }
  return 'just now';
}

const MONTHS = [
  'January',
  'February',
  'March',
  'April',
  'May',
  'June',
  'July',
  'August',
  'September',
  'October',
  'November',
  'December',
];

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

/** `2026-02` → `February 2026`. Anything else is returned unchanged. */
export function monthLabel(month: string): string {
  const match = /^(\d{4})-(\d{2})$/.exec(month);
  const name = match ? MONTHS[Number(match[2]) - 1] : undefined;
  return match && name ? `${name} ${match[1]}` : month;
}
