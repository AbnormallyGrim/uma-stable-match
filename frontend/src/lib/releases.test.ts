// Release notes data and "seen" tests.

import { describe, expect, it } from 'vitest';

import { hasMessage, t } from './i18n';
import {
  hasUnseenRelease,
  LATEST_BUILD,
  markReleaseSeen,
  RELEASE_SEEN_KEY,
  RELEASES,
} from './releases';

// Negative when build `a` is older than `b`.
function compareBuilds(a: string, b: string): number {
  const [x, y] = [a, b].map((build) => build.split('.').map(Number));
  return x!.map((part, i) => part - y![i]!).find((difference) => difference !== 0) ?? 0;
}

describe('RELEASES', () => {
  it('lists each build once, newest first', () => {
    for (const [index, release] of RELEASES.slice(0, -1).entries()) {
      expect(
        compareBuilds(release.build, RELEASES[index + 1]!.build),
        release.build,
      ).toBeGreaterThan(0);
    }
    expect(LATEST_BUILD).toBe(RELEASES[0]!.build);
  });

  it('never dates a release before the one under it', () => {
    for (const [index, release] of RELEASES.slice(0, -1).entries()) {
      expect(release.date >= RELEASES[index + 1]!.date, release.build).toBe(true);
    }
  });

  it('has a real date and some changes for every release', () => {
    for (const release of RELEASES) {
      expect(release.build, release.build).toMatch(/^\d+\.\d+\.\d+$/);
      expect(Number.isNaN(Date.parse(release.date)), release.build).toBe(false);
      expect(release.date, release.build).toMatch(/^\d{4}-\d{2}-\d{2}$/);
      expect(release.changes.length, release.build).toBeGreaterThan(0);
    }
  });

  it('only has hotfixes that fix', () => {
    for (const release of RELEASES.filter((r) => r.kind === 'hotfix')) {
      expect(
        release.changes.every((change) => change.kind === 'fixed'),
        release.build,
      ).toBe(true);
    }
  });

  it('starts every hotfix title with Fixed', () => {
    for (const change of RELEASES.filter((r) => r.kind === 'hotfix').flatMap((r) => r.changes)) {
      expect(t(change.title), change.title).toMatch(/^Fixed /);
    }
  });

  it('writes every change in a message', () => {
    for (const change of RELEASES.flatMap((release) => release.changes)) {
      expect(hasMessage(change.title), change.title).toBe(true);
      expect(hasMessage(change.body), change.body).toBe(true);
    }
  });
});

function memoryStorage(): Storage {
  const values = new Map<string, string>();
  return {
    getItem: (key) => values.get(key) ?? null,
    setItem: (key, value) => void values.set(key, value),
  } as Storage;
}

describe('seen releases', () => {
  it('is unseen until the visitor opens the notes at the newest build', () => {
    const storage = memoryStorage();
    expect(hasUnseenRelease(storage)).toBe(true);
    markReleaseSeen(storage);
    expect(storage.getItem(RELEASE_SEEN_KEY)).toBe(LATEST_BUILD);
    expect(hasUnseenRelease(storage)).toBe(false);
  });

  it('is unseen again after a newer build', () => {
    const storage = memoryStorage();
    storage.setItem(RELEASE_SEEN_KEY, '0.0.1');
    expect(hasUnseenRelease(storage)).toBe(true);
  });

  it('counts missing or blocked storage as seen, and ignores write errors', () => {
    const blocked = {
      getItem: () => {
        throw new Error('blocked');
      },
      setItem: () => {
        throw new Error('blocked');
      },
    } as unknown as Storage;
    expect(hasUnseenRelease(undefined)).toBe(false);
    expect(hasUnseenRelease(blocked)).toBe(false);
    expect(() => markReleaseSeen(blocked)).not.toThrow();
    expect(() => markReleaseSeen(undefined)).not.toThrow();
  });
});
