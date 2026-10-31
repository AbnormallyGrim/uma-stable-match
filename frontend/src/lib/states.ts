/** User-facing messages for search errors and empty results. */

import { ApiError, type Recommendations } from './api';
import { daysIntoGameMonth, gameMonth, monthLabel } from './format';
import { t } from './i18n';
import { discordNote } from './search';

/** A friendly error: a short title, what to do, and whether retrying can help. */
export interface ErrorMessage {
  title: string;
  detail: string;
  canRetry: boolean;
}

// The text is looked up when it's read, so a message on screen follows a language switch.
function message(title: () => string, detail: () => string, canRetry: boolean): ErrorMessage {
  return {
    get title() {
      return title();
    },
    get detail() {
      return detail();
    },
    canRetry,
  };
}

const generic = (detail: () => string = () => t('error.generic.detail')) =>
  message(() => t('error.generic.title'), detail, true);

/** Shown when the first scan is still running after every retry. */
export function scanStillLoading(): ErrorMessage {
  return message(
    () => t('error.scanStillLoading.title'),
    () => t('error.scanStillLoading.detail'),
    true,
  );
}

/** A friendly message for a failed search. */
export function describeError(error: unknown): ErrorMessage {
  if (!(error instanceof ApiError)) {
    return generic();
  }
  switch (error.kind) {
    case 'network':
      return message(
        () => t('error.network.title'),
        () => t('error.network.detail'),
        true,
      );
    case 'timeout':
      return message(
        () => t('error.timeout.title'),
        () => t('error.generic.detail'),
        true,
      );
    case 'invalid-response':
      return generic(() => t('error.invalidResponse.detail'));
    case 'human-check':
      return message(
        () => t('error.humanCheck.title'),
        () => error.message,
        true,
      );
    case 'http':
      return describeStatus(error);
  }
}

function describeStatus(error: ApiError): ErrorMessage {
  switch (error.status) {
    case 400:
      return message(
        () => t('error.badSearch.title'),
        () => error.message,
        false,
      );
    case 404:
      return message(
        () => t('error.trainerNotFound.title'),
        () => t('error.trainerNotFound.detail'),
        false,
      );
    case 422:
      return message(
        () => t('error.notEnoughData.title'),
        () => t('error.notEnoughData.detail'),
        false,
      );
    case 502:
      return message(
        () => t('error.umamoe.title'),
        () => t('error.umamoe.detail'),
        true,
      );
    default:
      return generic(() => error.message);
  }
}

/** A friendly message for a club page that failed to load. */
export function describeClubError(error: unknown): ErrorMessage {
  // The server's 404 for a club, rather than for the trainer.
  if (error instanceof ApiError && error.status === 404 && error.message.startsWith('club not')) {
    return message(
      () => t('clubPage.notLoaded.title'),
      () => t('clubPage.notLoaded.detail'),
      true,
    );
  }
  return describeError(error);
}

/** Suggestions for widening a search that found nothing. */
export function emptyHints(options: Recommendations['options']): string[] {
  const hints: string[] = [];
  if (options.has_discord) {
    hints.push(t('hint.discord'));
  }
  if (options.playstyles !== null) {
    hints.push(t('hint.playstyle'));
  }
  if (!options.include_closed) {
    hints.push(t('hint.closed'));
  }
  if (options.balance_weight !== null && options.balance_weight > 0) {
    hints.push(t('hint.balance'));
  }
  return hints;
}

/** Warns that join style or playstyle filters make matches less close, or `null` without them. */
export function filterNotice(options: Recommendations['options']): string | null {
  const joinStyles = options.join_styles;
  const narrowed =
    (joinStyles !== null && !(joinStyles.includes('open') && joinStyles.includes('approval'))) ||
    options.playstyles !== null;
  return narrowed ? t('notice.filtered') : null;
}

/** Says how many shortlisted clubs are still loading, or `null` when none are. */
export function pendingNotice(coverage: Recommendations['coverage']): string | null {
  const pending = coverage.pending;
  if (pending <= 0) {
    return null;
  }
  return t('notice.pending', { n: pending });
}

/** Days into a game month while the server blends club paces with last month's. */
export const BLENDED_DAYS = 5;

/**
 * Says when results use last month's data, or blend it in early in the game month, or `null`
 * otherwise.
 */
export function monthNotice(result: Pick<Recommendations, 'month' | 'updated_at'>): string | null {
  if (result.month === null) {
    return null;
  }
  if (result.month !== gameMonth(result.updated_at)) {
    return t('notice.lastMonth', { month: monthLabel(result.month) });
  }
  const days = daysIntoGameMonth(result.updated_at);
  return days !== null && days < BLENDED_DAYS ? t('notice.blended') : null;
}

/** Every notice for a result, the month first. */
export function resultNotices(result: Recommendations): string[] {
  return [
    filterNotice(result.options),
    result.options.has_discord ? discordNote() : null,
    monthNotice(result),
    pendingNotice(result.coverage),
  ].filter((notice): notice is string => notice !== null);
}
