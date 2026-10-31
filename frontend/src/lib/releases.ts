/** The release notes: what changed for visitors, newest first, and remembering the last one seen. */

import type { MessageKey } from './i18n';

/** The sort of change: something new, something that works better, or a bug that's gone. */
export type ChangeKind = 'new' | 'improved' | 'fixed';

/** An update adds or improves things; a hotfix only fixes a bug. */
export type ReleaseKind = 'update' | 'hotfix';

/** One change, as the messages that describe it. */
export interface Change {
  kind: ChangeKind;
  title: MessageKey;
  body: MessageKey;
}

/** One release. */
export interface Release {
  /** `major.minor.patch`: a new feature raises the minor number, a hotfix the patch number. */
  build: string;
  /** The day it went live, as `YYYY-MM-DD`. */
  date: string;
  kind: ReleaseKind;
  changes: Change[];
}

/** Every published release, newest first. See the frontend README before adding one. */
export const RELEASES: Release[] = [
  {
    build: '1.3.0',
    date: '2026-10-31',
    kind: 'update',
    changes: [
      {
        kind: 'new',
        title: 'releases.clubPage.title',
        body: 'releases.clubPage.body',
      },
    ],
  },
  {
    build: '1.2.0',
    date: '2026-10-30',
    kind: 'update',
    changes: [
      {
        kind: 'improved',
        title: 'releases.earlyMonth.title',
        body: 'releases.earlyMonth.body',
      },
    ],
  },
  {
    build: '1.1.0',
    date: '2026-10-24',
    kind: 'update',
    changes: [
      {
        kind: 'new',
        title: 'releases.yourClub.title',
        body: 'releases.yourClub.body',
      },
      {
        kind: 'new',
        title: 'releases.fullClubs.title',
        body: 'releases.fullClubs.body',
      },
      {
        kind: 'new',
        title: 'releases.filters.title',
        body: 'releases.filters.body',
      },
    ],
  },
  {
    build: '1.0.1',
    date: '2026-10-23',
    kind: 'hotfix',
    changes: [
      {
        kind: 'fixed',
        title: 'releases.movedClubs.title',
        body: 'releases.movedClubs.body',
      },
    ],
  },
];

/** The build number of the newest release. */
export const LATEST_BUILD = RELEASES[0]!.build;

/** Local storage key, set to the newest build the visitor has opened the release notes at. */
export const RELEASE_SEEN_KEY = 'uma-stable-match:release-seen';

/** Whether there's a release the visitor hasn't seen. Blocked storage counts as seen. */
export function hasUnseenRelease(storage: Storage | undefined): boolean {
  try {
    return storage !== undefined && storage.getItem(RELEASE_SEEN_KEY) !== LATEST_BUILD;
  } catch {
    return false;
  }
}

/** Remembers that the visitor has seen the newest release. Storage errors are ignored. */
export function markReleaseSeen(storage: Storage | undefined) {
  try {
    storage?.setItem(RELEASE_SEEN_KEY, LATEST_BUILD);
  } catch {
    return;
  }
}
