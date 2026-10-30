// Release notes modal tests. The releases are made up.

import { fireEvent, render, screen, within } from '@testing-library/svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';

import { language } from './language.svelte';
import type { Release } from './releases';
import ReleaseNotes from './ReleaseNotes.svelte';

const releases: Release[] = [
  {
    build: '2.1.0',
    date: '2026-03-05',
    kind: 'update',
    changes: [
      { kind: 'new', title: 'releases.yourClub.title', body: 'releases.yourClub.body' },
      { kind: 'improved', title: 'releases.earlyMonth.title', body: 'releases.earlyMonth.body' },
    ],
  },
  {
    build: '2.0.1',
    date: '2026-03-01',
    kind: 'hotfix',
    changes: [
      { kind: 'fixed', title: 'releases.movedClubs.title', body: 'releases.movedClubs.body' },
    ],
  },
];

afterEach(() => {
  language.choose('en');
  language.japaneseEnabled = false;
  document.documentElement.style.overflow = '';
});

function setup() {
  const onclose = vi.fn();
  render(ReleaseNotes, { releases, onclose });
  return onclose;
}

describe('ReleaseNotes', () => {
  it('opens as a labelled dialog with the close button focused', async () => {
    setup();
    const dialog = screen.getByRole('dialog', { name: "What's new" });
    expect(dialog).toHaveAttribute('aria-modal', 'true');
    expect(dialog).toHaveAccessibleDescription('Every update to Uma Stable Match, newest first.');
    await vi.waitFor(() => expect(screen.getByRole('button', { name: 'Close' })).toHaveFocus());
  });

  it('names the current build', () => {
    setup();
    expect(screen.getByText('Build 2.1.0')).toBeInTheDocument();
  });

  it('lists the releases newest first, with kind, date and changes', () => {
    setup();
    const items = screen
      .getAllByRole('heading', { level: 3 })
      .map((heading) => heading.textContent);
    expect(items).toEqual(['2.1.0', '2.0.1']);

    const [latest, hotfix] = screen.getAllByRole('listitem').filter((li) => li.matches('.release'));
    expect(within(latest!).getByText('Update')).toBeInTheDocument();
    expect(within(latest!).getByText('Latest')).toBeInTheDocument();
    expect(within(latest!).getByText('March 5, 2026')).toHaveAttribute('datetime', '2026-03-05');
    expect(within(latest!).getByText('New')).toBeInTheDocument();
    expect(within(latest!).getByText('Improved')).toBeInTheDocument();
    expect(
      within(latest!).getByRole('heading', { name: 'Compare your current club' }),
    ).toBeInTheDocument();

    expect(within(hotfix!).getByText('Hotfix')).toBeInTheDocument();
    expect(within(hotfix!).queryByText('Latest')).not.toBeInTheDocument();
    expect(within(hotfix!).getByText('Fixed')).toBeInTheDocument();
  });

  it('shows the Japanese notes when Japanese is on', () => {
    language.japaneseEnabled = true;
    language.choose('ja');
    setup();
    expect(screen.getByRole('dialog', { name: '更新情報' })).toBeInTheDocument();
    expect(screen.getByText('2026年3月5日')).toBeInTheDocument();
    expect(screen.getByText('緊急修正')).toBeInTheDocument();
  });

  it('closes with the button, Escape or a click outside, but not a click inside', async () => {
    const onclose = setup();
    await fireEvent.click(screen.getByRole('dialog'));
    expect(onclose).not.toHaveBeenCalled();
    await fireEvent.click(screen.getByRole('button', { name: 'Close' }));
    expect(onclose).toHaveBeenCalledTimes(1);
    await fireEvent.keyDown(window, { key: 'Escape' });
    expect(onclose).toHaveBeenCalledTimes(2);
    await fireEvent.click(screen.getByRole('dialog').parentElement!);
    expect(onclose).toHaveBeenCalledTimes(3);
  });

  it('keeps Tab inside the dialog', async () => {
    setup();
    const close = screen.getByRole('button', { name: 'Close' });
    const scroll = screen.getByRole('region', { name: "What's new" });
    await vi.waitFor(() => expect(close).toHaveFocus());
    await fireEvent.keyDown(window, { key: 'Tab', shiftKey: true });
    expect(scroll).toHaveFocus();
    await fireEvent.keyDown(window, { key: 'Tab' });
    expect(close).toHaveFocus();
  });

  it('stops the page scrolling behind it, and gives that back', () => {
    const { unmount } = render(ReleaseNotes, { releases, onclose: () => {} });
    expect(document.documentElement.style.overflow).toBe('hidden');
    unmount();
    expect(document.documentElement.style.overflow).toBe('');
  });
});
