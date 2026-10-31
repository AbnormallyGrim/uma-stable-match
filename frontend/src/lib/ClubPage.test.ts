// ClubPage component tests with a stubbed loader. All data is made up.

import { fireEvent, render, screen, within } from '@testing-library/svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';

import { ApiError, type ClubDetails, type fetchClub } from './api';
import ClubPage from './ClubPage.svelte';
import { language } from './language.svelte';
import { rememberSearch } from './route';
import { club } from './test-data';

const M = 1_000_000;

// Cumulative fans from 100M for five days at a made-up pace.
const fans = (pace: number) => Array.from({ length: 5 }, (_, d) => 100 * M + pace * d);

function details(overrides: Partial<ClubDetails> = {}): ClubDetails {
  return {
    club: club({ name: 'Page Club', comment: 'Made-up description' }),
    month: '2026-03',
    updated_at: '2026-03-19T15:00:00Z',
    median_per_day: 6.5 * M,
    members: [9 * M, 7 * M, 6 * M, 4 * M, null].map((pace, i) => ({
      name: i === 3 ? null : `Made-up Trainer ${'ABCDE'[i]}`,
      pace_per_day: pace,
      daily_fans: fans(pace ?? 0),
    })),
    trainer: null,
    ...overrides,
  };
}

const trainer = {
  trainer_id: 123456789012,
  pace_per_day: 6.5 * M,
  daily_fans: null,
  in_club: false,
  your_place: 3,
  club_size: 6,
};

function setup(load: typeof fetchClub, trainerId: string | null = null) {
  return render(ClubPage, { route: { circleId: '1', trainerId }, load });
}

const resolving = (value: ClubDetails) => vi.fn<typeof fetchClub>(() => Promise.resolve(value));

// Each member card's place, name, pace and gap, as shown.
function cards() {
  return [...document.querySelectorAll('.roster li')].map((card) =>
    [...card.querySelectorAll('.place, .name, .pace, .gap')].map((part) =>
      part.textContent?.replace(/\s+/g, ' ').trim(),
    ),
  );
}

afterEach(() => {
  sessionStorage.clear();
  vi.unstubAllGlobals();
  language.choose('en');
  language.japaneseEnabled = false;
});

describe('ClubPage', () => {
  it('shows the club and its members, fastest first', async () => {
    const load = resolving(details());
    setup(load);
    expect(screen.getByRole('status')).toHaveTextContent('Loading the club…');
    expect(screen.getByRole('status').querySelector('.track')).toBeInTheDocument();

    const link = await screen.findByRole('link', { name: 'Page Club' });
    expect(link).toHaveAttribute('href', 'https://uma.moe/circles/1');
    expect(screen.getByRole('heading', { level: 1 })).toHaveTextContent('Page Club');
    expect(screen.getByText('Made-up description')).toBeInTheDocument();
    expect(screen.getByText('6.5M/day')).toBeInTheDocument();
    expect(
      screen.getByText(/5 current members, fastest first\. Data for March 2026/),
    ).toBeVisible();
    expect(cards()).toEqual([
      ['1', 'Made-up Trainer A', '9.0M/day'],
      ['2', 'Made-up Trainer B', '7.0M/day'],
      ['3', 'Made-up Trainer C', '6.0M/day'],
      ['4', 'Unknown trainer', '4.0M/day'],
      ['–', 'Made-up Trainer E', 'Too new to rank'],
    ]);
    expect(load).toHaveBeenCalledWith('1', null, expect.anything());
    expect(document.title).toBe('Page Club · Uma Stable Match');
  });

  it('adds the trainer as a highlighted card where they would rank', async () => {
    const load = resolving(details({ trainer }));
    const { container } = setup(load, '123456789012');

    await screen.findByRole('heading', { name: 'Members' });
    expect(load).toHaveBeenCalledWith('1', '123456789012', expect.anything());
    expect(cards()).toEqual([
      ['1', 'Made-up Trainer A', '9.0M/day', '+2.5M/day vs you'],
      ['2', 'Made-up Trainer B', '7.0M/day', '+500K/day vs you'],
      ['3', 'You', '6.5M/day'],
      ['4', 'Made-up Trainer C', '6.0M/day', '−500K/day vs you'],
      ['5', 'Unknown trainer', '4.0M/day', '−2.5M/day vs you'],
      ['–', 'Made-up Trainer E', 'Too new to rank'],
    ]);
    const you = container.querySelectorAll('.roster li')[2]!.firstElementChild;
    expect(you).toHaveClass('you');
    expect(you?.tagName).not.toBe('BUTTON');
    expect(screen.getByText('3 of 6')).toBeInTheDocument();
    expect(screen.queryByText(/This is your club/)).not.toBeInTheDocument();
  });

  it('explains what the page shows when the trainer is placed', async () => {
    setup(resolving(details({ trainer })), '123456789012');
    expect(
      await screen.findByText(/See how you'd fit in this club before you apply/),
    ).toBeVisible();
  });

  it('suggests a trainer search when the club is shown alone', async () => {
    setup(resolving(details()));
    expect(await screen.findByText(/Search by trainer ID to see where you'd rank/)).toBeVisible();
  });

  it('says when it is the trainer’s own club', async () => {
    setup(resolving(details({ trainer: { ...trainer, in_club: true } })), '123456789012');
    expect(await screen.findByText(/This is your club/)).toBeInTheDocument();
  });

  it('explains a club that is not loaded and retries', async () => {
    const load = vi
      .fn<typeof fetchClub>()
      .mockRejectedValueOnce(new ApiError('http', 'club not loaded yet; try again later', 404))
      .mockResolvedValueOnce(details());
    setup(load);

    const alert = await screen.findByRole('alert');
    expect(alert).toHaveTextContent("This club isn't loaded yet");
    await fireEvent.click(within(alert).getByRole('button', { name: 'Try again' }));
    expect(await screen.findByRole('heading', { name: 'Members' })).toBeInTheDocument();
    expect(load).toHaveBeenCalledTimes(2);
  });

  it('offers no retry for an unknown trainer', async () => {
    setup(
      vi.fn<typeof fetchClub>(() => Promise.reject(new ApiError('http', 'trainer not found', 404))),
      '123456789012',
    );
    const alert = await screen.findByRole('alert');
    expect(alert).toHaveTextContent("We couldn't find that trainer");
    expect(within(alert).queryByRole('button')).not.toBeInTheDocument();
  });

  it('shows the chart above the members', async () => {
    setup(resolving(details()));
    const chart = await screen.findByRole('heading', { name: 'Pace this month' });
    const members = screen.getByRole('heading', { name: 'Members' });
    expect(chart.compareDocumentPosition(members) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
  });

  it('lists members in numbered order', async () => {
    const { container } = setup(resolving(details()));
    await screen.findByRole('heading', { name: 'Members' });
    expect(container.querySelector('ol.roster')).toBeInTheDocument();
  });

  it('highlights a member on the chart when their card is selected, and clears it again', async () => {
    const { container } = setup(resolving(details()));
    const name = await screen.findByRole('button', { name: /Made-up Trainer B/ });
    expect(name).toHaveAttribute('aria-pressed', 'false');
    expect(container.querySelector('path.chosen')).toBeNull();

    await fireEvent.click(name);
    expect(name).toHaveAttribute('aria-pressed', 'true');
    expect(name).toHaveClass('chosen');
    expect(container.querySelector('path.chosen')).toHaveAttribute('d');
    expect(container.querySelector('.legend')).toHaveTextContent('Made-up Trainer B');

    await fireEvent.click(screen.getByRole('button', { name: /Made-up Trainer C/ }));
    expect(name).toHaveAttribute('aria-pressed', 'false');
    expect(container.querySelectorAll('.card.chosen')).toHaveLength(1);

    await fireEvent.click(screen.getByRole('button', { name: /Made-up Trainer C/ }));
    expect(container.querySelector('path.chosen')).toBeNull();
  });

  it('offers the language button and the saved language when the server has Japanese on', async () => {
    language.choose('ja');
    vi.stubGlobal(
      'fetch',
      vi.fn(() =>
        Promise.resolve(
          new Response(JSON.stringify({ japanese: true }), {
            headers: { 'Content-Type': 'application/json' },
          }),
        ),
      ),
    );
    setup(resolving(details()));
    expect(await screen.findByRole('link', { name: '← 検索に戻る' })).toBeInTheDocument();
    expect(document.documentElement.lang).toBe('ja');
    expect(screen.getByRole('button', { name: /English/ })).toBeInTheDocument();
  });

  it('links to a new search when it was not opened from one', () => {
    setup(resolving(details()));
    expect(screen.getByRole('link', { name: '← Back to search' })).toHaveAttribute('href', '/');
  });

  it('links back to the search it was opened from', () => {
    const search = `${window.location.origin}/?trainer_id=123456789012`;
    rememberSearch(sessionStorage, search);
    setup(resolving(details()));
    expect(screen.getByRole('link', { name: '← Back to search' })).toHaveAttribute('href', search);
  });

  it('ignores a remembered address on another site', () => {
    rememberSearch(sessionStorage, 'https://example.com/?trainer_id=123456789012');
    setup(resolving(details()));
    expect(screen.getByRole('link', { name: '← Back to search' })).toHaveAttribute('href', '/');
  });
});
