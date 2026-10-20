/** User-facing messages for search errors and empty results. */

import { ApiError, type Recommendations } from './api';
import { gameMonth, monthLabel } from './format';

/** A friendly error: a short title, what to do, and whether retrying can help. */
export interface ErrorMessage {
  title: string;
  detail: string;
  canRetry: boolean;
}

/** Shown when the first scan is still running after every retry. */
export const SCAN_STILL_LOADING: ErrorMessage = {
  title: "Club data still isn't ready",
  detail: 'The first scan is taking longer than usual. Try again in a minute.',
  canRetry: true,
};

/** A friendly message for a failed search. */
export function describeError(error: unknown): ErrorMessage {
  if (!(error instanceof ApiError)) {
    return { title: 'Something went wrong', detail: 'Try again in a moment.', canRetry: true };
  }
  switch (error.kind) {
    case 'network':
      return {
        title: "Couldn't reach the server",
        detail: 'Check your connection and try again.',
        canRetry: true,
      };
    case 'timeout':
      return {
        title: 'The server took too long to answer',
        detail: 'Try again in a moment.',
        canRetry: true,
      };
    case 'invalid-response':
      return {
        title: 'Something went wrong',
        detail: 'The server sent an unexpected response. Try again in a moment.',
        canRetry: true,
      };
    case 'http':
      return describeStatus(error);
  }
}

function describeStatus(error: ApiError): ErrorMessage {
  switch (error.status) {
    case 400:
      return { title: "That search didn't work", detail: error.message, canRetry: false };
    case 404:
      return {
        title: "We couldn't find that trainer",
        detail:
          "Check the trainer ID. It's 9 to 12 digits, shown in-game under Menu → Profile. The trainer also needs to be in a club that uma.moe tracks.",
        canRetry: false,
      };
    case 422:
      return {
        title: 'Not enough fan data yet',
        detail:
          'uma.moe needs at least 3 days of fan data this month to work out your pace. Try again in a few days, or search by daily pace instead.',
        canRetry: false,
      };
    case 502:
      return {
        title: "Couldn't reach uma.moe",
        detail: "The server couldn't get your data from uma.moe. Try again in a moment.",
        canRetry: true,
      };
    default:
      return { title: 'Something went wrong', detail: error.message, canRetry: true };
  }
}

/** Suggestions for widening a search that found nothing. */
export function emptyHints(options: Recommendations['options']): string[] {
  const hints: string[] = [];
  if (!options.include_closed) {
    hints.push("Include closed clubs to see clubs that aren't recruiting right now.");
  }
  if (options.balance_weight !== null && options.balance_weight > 0) {
    hints.push('Lower the balance strength or turn balance scoring off.');
  }
  return hints;
}

/** Says how many shortlisted clubs are still loading, or `null` when none are. */
export function pendingNotice(coverage: Recommendations['coverage']): string | null {
  const pending = coverage.pending;
  if (pending <= 0) {
    return null;
  }
  const clubs = pending === 1 ? 'club is' : 'clubs are';
  return `${pending} more ${clubs} still loading. Search again in a minute for complete results.`;
}

/** Says when results use last month's data, or `null` when they use the current game month. */
export function monthNotice(result: Pick<Recommendations, 'month' | 'updated_at'>): string | null {
  if (result.month === null || result.month === gameMonth(result.updated_at)) {
    return null;
  }
  return `These results are based on last month's data (${monthLabel(result.month)}), because the new game month has only just started.`;
}

/** Every notice for a result, the month first. */
export function resultNotices(result: Recommendations): string[] {
  return [monthNotice(result), pendingNotice(result.coverage)].filter(
    (notice): notice is string => notice !== null,
  );
}
