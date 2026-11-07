/** A club's fan requirement, such as 8M a week, and what it means on the chart. */

/** How often the requirement applies. */
export type Interval = 'day' | 'week' | 'month';

/** The intervals a leader can choose from. */
export const INTERVALS: Interval[] = ['day', 'week', 'month'];

/** Fans every `per`. */
export interface Requirement {
  fans: number;
  per: Interval;
}

/** Days in the game month `YYYY-MM` (`null` counts as 30), which a monthly requirement spreads over. */
export function daysInMonth(month: string | null): number {
  const match = month === null ? null : /^(\d{4})-(\d{2})$/.exec(month);
  return match ? new Date(Date.UTC(Number(match[1]), Number(match[2]), 0)).getUTCDate() : 30;
}

/** The fans a day the requirement comes to. */
export function perDay(requirement: Requirement, month: string | null): number {
  const days = { day: 1, week: 7, month: daysInMonth(month) }[requirement.per];
  return requirement.fans / days;
}

/** The requirement in an address's `need` (fans) and `per`, or `null` if either is unusable. */
export function parseRequirement(need: string | null, per: string | null): Requirement | null {
  const fans = need !== null && /^\d{1,12}$/.test(need) ? Number(need) : 0;
  const interval = INTERVALS.find((candidate) => candidate === per);
  return fans > 0 && interval ? { fans, per: interval } : null;
}

const KEY = 'uma-stable-match:requirement:';

/** The requirement remembered in this browser for club `clubId`, if any. */
export function rememberedRequirement(
  storage: Storage | undefined,
  clubId: string,
): Requirement | null {
  try {
    const saved: unknown = JSON.parse(storage?.getItem(`${KEY}${clubId}`) ?? 'null');
    if (typeof saved !== 'object' || saved === null) return null;
    const { fans, per } = saved as Record<string, unknown>;
    return parseRequirement(String(fans), String(per));
  } catch {
    return null;
  }
}

/** Remembers `requirement` for club `clubId` in this browser, or forgets it when `null`. */
export function rememberRequirement(
  storage: Storage | undefined,
  clubId: string,
  requirement: Requirement | null,
): void {
  try {
    if (requirement === null) {
      storage?.removeItem(`${KEY}${clubId}`);
    } else {
      storage?.setItem(`${KEY}${clubId}`, JSON.stringify(requirement));
    }
  } catch {
    // Without storage, the requirement lasts until the page closes.
  }
}
