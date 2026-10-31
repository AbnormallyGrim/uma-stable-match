/** The StrawPoll polls on the wishlist page. */

import type { MessageKey } from './i18n';

/** One embedded poll. */
export interface Poll {
  /** The ID in the poll's StrawPoll address. */
  id: string;
  /** The message naming the poll for screen readers. */
  title: MessageKey;
  /** The frame's height in pixels, enough to show the whole poll without a gap below it. */
  height: number;
}

/** The polls, in the order they show. */
export const POLLS: Poll[] = [
  { id: '7MZ0k5l2mgo', title: 'wishlist.poll.dashboard', height: 440 },
  { id: 'poy9kMvNAgJ', title: 'wishlist.poll.discord', height: 490 },
];

/** The address of a poll's embeddable page. */
export const embedUrl = (id: string): string => `https://strawpoll.com/embed/${id}`;
