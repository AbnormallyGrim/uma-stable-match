/** The product tours' steps, and remembering whether a visitor has seen them. */

import { t } from './i18n';

/** One step: what it highlights, if anything, and what it says. */
export interface TourStep {
  /** `data-tour` value of the element to highlight. Without one, the step is centred. */
  target?: string;
  title: string;
  body: string;
}

/** Local storage key, set once the search tour is finished or skipped. */
export const TOUR_KEY = 'uma-stable-match:tour-seen';

/** Local storage key, set once the board tour is finished or skipped. */
export const BOARD_TOUR_KEY = 'uma-stable-match:board-tour-seen';

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

/**
 * The board's steps: why to post, what the board shows, the filters and the button that posts.
 * With `form`, a last step opens the listing form, which the page opens on seeing its target.
 */
export function boardTourSteps(form: boolean): TourStep[] {
  return [
    {
      title: t('tour.board.welcome.title'),
      body: t('tour.board.welcome.body'),
    },
    {
      target: 'board',
      title: t('tour.board.list.title'),
      body: t('tour.board.list.body'),
    },
    {
      target: 'board-filters',
      title: t('tour.board.filters.title'),
      body: t('tour.board.filters.body'),
    },
    {
      target: 'board-post',
      title: t('tour.board.post.title'),
      body: t('tour.board.post.body'),
    },
    ...(form
      ? [
          {
            target: 'ad-form',
            title: t('tour.board.form.title'),
            body: t('tour.board.form.body'),
          },
        ]
      : []),
  ];
}

/** Whether a tour was finished or skipped on this device. Blocked storage counts as seen. */
export function hasSeenTour(storage: Storage | undefined, key = TOUR_KEY): boolean {
  try {
    return storage === undefined || storage.getItem(key) !== null;
  } catch {
    return true;
  }
}

/** Remembers that a tour was finished or skipped. Storage errors are ignored. */
export function markTourSeen(storage: Storage | undefined, key = TOUR_KEY) {
  try {
    storage?.setItem(key, '1');
  } catch {
    return;
  }
}
