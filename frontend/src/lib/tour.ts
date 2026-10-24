/** The product tour's steps, and remembering whether a visitor has seen it. */

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
export const TOUR_STEPS: TourStep[] = [
  {
    title: 'Welcome to Uma Stable Match',
    body: 'Find clubs where your daily fan gain fits and members pull their weight. This quick tour shows you around.',
  },
  {
    target: 'search-by',
    title: 'Search by trainer ID or pace',
    body: 'Pick Trainer ID to look up your pace automatically, or Daily pace to type it in yourself.',
  },
  {
    target: 'search-field',
    title: 'Enter your details',
    body: 'Your trainer ID is the 9 to 12 digit number on your profile, under Menu → Profile Trainer Card. For daily pace, enter the fans you gain a day in millions, such as 7.6.',
  },
  {
    target: 'balance',
    title: 'Balance scoring',
    body: 'Turn this on to rank clubs lower when a few members carry everyone else. The slider sets how strongly.',
  },
  {
    target: 'filters',
    title: 'Narrow your search',
    body: 'Include full clubs to compare against, keep only clubs that mention Discord, or pick one join style or playstyle. Filtering leaves fewer clubs near your pace, so matches can be a little looser.',
  },
  {
    target: 'find',
    title: 'Find clubs',
    body: 'Run the search. It is saved in the page address, so you can bookmark or share it.',
  },
  {
    title: 'Read your results',
    body: "You'll get a summary, your top picks as cards, and the top results in a table you can search and sort. Searching by trainer ID also compares your current club. Select a club's name to open its uma.moe page and contact the leader.",
  },
  {
    target: 'how-to',
    title: 'How to use',
    body: 'Step-by-step instructions, including where to find your trainer ID and pace, are here whenever you need them.',
  },
  {
    target: 'theme',
    title: 'Light or dark',
    body: 'Follow your device, or pick the light or dark theme.',
  },
  {
    target: 'help',
    title: 'Take the tour again',
    body: 'Select the question mark to bring this tour back any time.',
  },
];

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
