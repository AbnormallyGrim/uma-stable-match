/** The product tour's steps, and remembering whether a visitor has seen it. */

import { t } from './i18n';

/** One step: what it highlights, if anything, and what it says. */
export interface TourStep {
  /** `data-tour` value of the element to highlight. Without one, the step is centred. */
  target?: string;
  title: string;
  body: string;
}

/** Local storage key, set once the tour is finished or skipped. */
export const TOUR_KEY = 'uma-stable-match:tour-seen';

/** The steps, in page order. Keep the labels they quote in step with the page. */
export function tourSteps(): TourStep[] {
  return [
    {
      title: t('tour.welcome.title'),
      body: t('tour.welcome.body'),
    },
    {
      target: 'search-by',
      title: t('tour.searchBy.title'),
      body: t('tour.searchBy.body'),
    },
    {
      target: 'search-field',
      title: t('tour.field.title'),
      body: t('tour.field.body'),
    },
    {
      target: 'balance',
      title: t('tour.balance.title'),
      body: t('tour.balance.body'),
    },
    {
      target: 'filters',
      title: t('tour.filters.title'),
      body: t('tour.filters.body'),
    },
    {
      target: 'find',
      title: t('tour.find.title'),
      body: t('tour.find.body'),
    },
    {
      title: t('tour.results.title'),
      body: t('tour.results.body'),
    },
    {
      target: 'how-to',
      title: t('tour.howTo.title'),
      body: t('tour.howTo.body'),
    },
    {
      target: 'theme',
      title: t('tour.theme.title'),
      body: t('tour.theme.body'),
    },
    {
      target: 'help',
      title: t('tour.help.title'),
      body: t('tour.help.body'),
    },
  ];
}

/** Whether the tour was finished or skipped on this device. Blocked storage counts as seen. */
export function hasSeenTour(storage: Storage | undefined): boolean {
  try {
    return storage === undefined || storage.getItem(TOUR_KEY) !== null;
  } catch {
    return true;
  }
}

/** Remembers that the tour was finished or skipped. Storage errors are ignored. */
export function markTourSeen(storage: Storage | undefined) {
  try {
    storage?.setItem(TOUR_KEY, '1');
  } catch {
    return;
  }
}
