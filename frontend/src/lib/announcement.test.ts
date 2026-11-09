// Announcement tests: who gets it, and the dialog itself.

import { fireEvent, render, screen } from '@testing-library/svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';

import {
  ANNOUNCEMENT_KEY,
  hasSeenAnnouncement,
  isAnnouncedPage,
  markAnnouncementSeen,
  shouldAnnounce,
} from './announcement';
import Announcement from './Announcement.svelte';

afterEach(() => localStorage.clear());

describe('shouldAnnounce', () => {
  it('points visitors on other pages to the new ones', () => {
    for (const path of ['/club/123', '/wishlist', '/settings']) {
      expect(shouldAnnounce(path, localStorage)).toBe(true);
    }
  });

  it('stays quiet on the new pages, including the old board address', () => {
    for (const path of ['/applications', '/applications/trainer', '/recruitment-board/']) {
      expect(isAnnouncedPage(path)).toBe(true);
      expect(shouldAnnounce(path, localStorage)).toBe(false);
    }
    expect(isAnnouncedPage('/applicationsfoo')).toBe(false);
  });

  it('stays quiet on the sign-in', () => {
    expect(shouldAnnounce('/login', localStorage)).toBe(false);
  });

  it('leaves the search page to wait for its tour, so it doesn’t rule it out', () => {
    expect(shouldAnnounce('/', localStorage)).toBe(true);
  });

  it('shows once', () => {
    markAnnouncementSeen(localStorage);
    expect(localStorage.getItem(ANNOUNCEMENT_KEY)).toBe('1');
    expect(shouldAnnounce('/wishlist', localStorage)).toBe(false);
  });

  it('counts blocked storage as seen, so it never repeats on every visit', () => {
    const blocked = {
      getItem: () => {
        throw new Error('blocked');
      },
      setItem: () => {
        throw new Error('blocked');
      },
    } as unknown as Storage;
    expect(hasSeenAnnouncement(blocked)).toBe(true);
    expect(hasSeenAnnouncement(undefined)).toBe(true);
    expect(() => markAnnouncementSeen(blocked)).not.toThrow();
  });
});

describe('Announcement', () => {
  it('links to both pages and counts as seen when it opens', () => {
    render(Announcement);
    expect(screen.getByRole('dialog', { name: 'What’s new' })).toBeInTheDocument();
    expect(screen.getByRole('link', { name: /Recruitment Board/ })).toHaveAttribute(
      'href',
      '/recruitment-board',
    );
    expect(screen.getByRole('link', { name: /Applications/ })).toHaveAttribute(
      'href',
      '/applications',
    );
    expect(hasSeenAnnouncement(localStorage)).toBe(true);
  });

  it('closes from its button', async () => {
    render(Announcement);
    await fireEvent.click(screen.getByRole('button', { name: 'Maybe later' }));
    await vi.waitFor(() => expect(screen.queryByRole('dialog')).not.toBeInTheDocument());
  });
});
