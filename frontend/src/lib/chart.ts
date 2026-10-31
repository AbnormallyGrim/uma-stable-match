/** The club page chart's numbers: daily gains or fans this month, per day, and the axis ticks. */

/** What the chart plots. */
export type ChartMode = 'daily' | 'month';

/** One line: a value for each day index, or `null` where there's no data. */
export type Series = (number | null)[];

/**
 * A cumulative series as the chart plots it. `daily` is each day's gain, averaged over any days
 * without data before it; `month` is the fans gained since the first day with data. `0` in
 * `fans` means no data.
 */
export function toSeries(fans: number[], mode: ChartMode): Series {
  const series: Series = fans.map(() => null);
  let first: number | null = null;
  let previous: { day: number; fans: number } | null = null;
  fans.forEach((value, day) => {
    if (value === 0) return;
    first ??= value;
    if (mode === 'month') {
      series[day] = value - first;
    } else if (previous) {
      series[day] = Math.max(0, (value - previous.fans) / (day - previous.day));
    }
    previous = { day, fans: value };
  });
  return series;
}

/** Fewest values on a day for the median line to show it. */
export const MIN_FOR_MEDIAN = 3;

/** The median of `series` on each day, where at least `MIN_FOR_MEDIAN` have a value. */
export function medianSeries(series: Series[], days: number): Series {
  return Array.from({ length: days }, (_, day) => {
    const values = series.map((s) => s[day]).filter((v): v is number => v != null);
    return values.length >= MIN_FOR_MEDIAN ? median(values) : null;
  });
}

/** The last day index that has a value in any series, or -1 if none do. */
export function lastDay(series: Series[]): number {
  let last = -1;
  for (const s of series) {
    s.forEach((v, day) => {
      if (v !== null && day > last) last = day;
    });
  }
  return last;
}

function median(values: number[]): number {
  const sorted = [...values].sort((a, b) => a - b);
  const mid = sorted.length >> 1;
  return sorted.length % 2 ? sorted[mid]! : (sorted[mid - 1]! + sorted[mid]!) / 2;
}

/** Round tick values from 0 to at least `max`, about `count` of them. */
export function ticks(max: number, count = 4): number[] {
  if (!(max > 0)) return [0];
  const rough = max / count;
  const power = 10 ** Math.floor(Math.log10(rough));
  const step = [1, 2, 2.5, 5, 10].map((m) => m * power).find((s) => s >= rough)!;
  const top = Math.ceil(max / step) * step;
  return Array.from({ length: Math.round(top / step) + 1 }, (_, i) => i * step);
}

/** A tick label: `0`, `500K`, `2M`, `2.5M`. */
export function tickLabel(value: number): string {
  if (value >= 1_000_000) return `${+(value / 1_000_000).toFixed(2)}M`;
  if (value >= 1000) return `${+(value / 1000).toFixed(1)}K`;
  return String(value);
}
