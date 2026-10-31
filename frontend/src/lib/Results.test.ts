// Results component tests. All data is made up.

import { render, screen, within } from '@testing-library/svelte';
import { describe, expect, it } from 'vitest';

import Results from './Results.svelte';
import { club, recommendations, yourClub } from './test-data';

describe('Results', () => {
  it('summarises the search', () => {
    const clubs = [
      club({ circle_id: 1, your_place: 9, club_size: 26 }),
      club({ circle_id: 2, your_place: 2, club_size: 20 }),
      club({ circle_id: 3, your_place: 12, club_size: 20 }),
    ];
    render(Results, {
      result: recommendations({
        total: 42,
        coverage: { considered: 4321, shortlisted: 120, scored: 42, pending: 0 },
        clubs,
      }),
    });
    const summary = screen.getByTestId('result-summary');
    expect(within(summary).getByText('7.6M/day')).toBeInTheDocument();
    expect(within(summary).getByText('4321')).toBeInTheDocument();
    expect(within(summary).getByText('2')).toBeInTheDocument();
    expect(within(summary).getByText('Top 35%')).toBeInTheDocument();
  });

  it("doesn't repeat the data age or the balance setting", () => {
    render(Results, {
      result: recommendations({
        options: {
          balance_weight: 0.5,
          include_closed: false,
          include_full: false,
          join_styles: null,
          playstyles: null,
          has_discord: false,
          limit: 50,
        },
      }),
    });
    const summary = screen.getByTestId('result-summary');
    expect(summary).not.toHaveTextContent('Data updated');
    expect(summary).not.toHaveTextContent('Weight 0.50');
  });

  it('lists up to ten top picks in order', () => {
    const clubs = Array.from({ length: 12 }, (_, i) =>
      club({ circle_id: i + 1, name: `Club ${i + 1}`, url: `https://uma.moe/circles/${i + 1}` }),
    );
    render(Results, { result: recommendations({ total: 12, clubs }) });
    const picks = screen.getByRole('list');
    const items = within(picks).getAllByRole('listitem');
    expect(items).toHaveLength(10);
    expect(within(items[0]!).getByRole('link', { name: 'Club 1' })).toBeInTheDocument();
    expect(within(items[9]!).getByRole('link', { name: 'Club 10' })).toBeInTheDocument();
    expect(within(picks).queryByText('Club 11')).not.toBeInTheDocument();
    expect(within(screen.getByRole('table')).getAllByRole('link', { name: /^Club / })).toHaveLength(
      12,
    );
  });

  it('heads the table with the number of results', () => {
    const clubs = Array.from({ length: 12 }, (_, i) => club({ circle_id: i + 1 }));
    render(Results, { result: recommendations({ total: 40, clubs }) });
    expect(screen.getByRole('heading', { name: 'Top 12 results' })).toBeInTheDocument();
  });

  it('heads a single result in the singular', () => {
    render(Results, { result: recommendations({ total: 1, clubs: [club()] }) });
    expect(screen.getByRole('heading', { name: 'Top 1 result' })).toBeInTheDocument();
  });

  it('explains missing full clubs and lower tiers, even with no matches', () => {
    render(Results, { result: recommendations() });
    const about = screen.getByTestId('results-about');
    expect(about).toHaveTextContent("Full clubs (30 of 30 members) aren't in the results");
    expect(about).toHaveTextContent('search by trainer ID to see how it compares.');
    expect(within(about).getByText('Why not a higher-tier club?')).toBeInTheDocument();
    expect(screen.queryByRole('table')).not.toBeInTheDocument();
  });

  it("shows the trainer's own club when the search found it", () => {
    render(Results, {
      result: recommendations({ total: 1, clubs: [club()], your_club: yourClub() }),
    });
    expect(screen.getByTestId('your-club')).toHaveTextContent('Home Club');
    expect(screen.getByTestId('results-about')).toHaveTextContent(
      "it's compared with your top pick below.",
    );
  });

  it('shows no own club card without one', () => {
    render(Results, { result: recommendations({ total: 1, clubs: [club()] }) });
    expect(screen.queryByTestId('your-club')).not.toBeInTheDocument();
  });

  it('leaves full clubs out of the top picks but keeps them in the table', () => {
    const clubs = [
      club({ circle_id: 1, name: 'Full Club', url: 'https://uma.moe/circles/1', open_spots: 0 }),
      club({ circle_id: 2, name: 'Open Club', url: 'https://uma.moe/circles/2' }),
    ];
    render(Results, { result: recommendations({ total: 2, clubs }) });
    const picks = within(screen.getByRole('list')).getAllByRole('listitem');
    expect(picks).toHaveLength(1);
    expect(picks[0]).toHaveTextContent('Open Club');
    const table = screen.getByRole('table');
    expect(within(table).getByRole('link', { name: 'Full Club' })).toBeInTheDocument();
    expect(within(table).getByText('Full')).toBeInTheDocument();
  });

  it('says when every match is full', () => {
    const clubs = [club({ open_spots: 0 })];
    render(Results, {
      result: recommendations({
        total: 1,
        clubs,
        options: {
          balance_weight: null,
          include_closed: false,
          include_full: true,
          join_styles: null,
          playstyles: null,
          has_discord: false,
          limit: 100,
        },
      }),
    });
    expect(screen.getByText(/Every matching club is full/)).toBeInTheDocument();
    expect(screen.getByRole('table')).toBeInTheDocument();
    expect(screen.getByTestId('results-about')).toHaveTextContent(
      'Full clubs (30 of 30 members) are in the table, but not the top picks',
    );
  });

  it('says the Discord filter is a best guess', () => {
    render(Results, {
      result: recommendations({
        options: {
          balance_weight: null,
          include_closed: false,
          include_full: false,
          join_styles: null,
          playstyles: null,
          has_discord: true,
          limit: 100,
        },
      }),
    });
    expect(screen.getByText(/Some clubs with a Discord won't show up/)).toBeInTheDocument();
  });

  it('says when nothing matched', () => {
    render(Results, { result: recommendations() });
    expect(screen.getByText('No clubs matched your search.')).toBeInTheDocument();
  });
});

describe('Results club page links', () => {
  const clubs = [1, 2].map((id) =>
    club({ circle_id: id, name: `Club ${id}`, url: `https://uma.moe/circles/${id}` }),
  );

  it('links every pick, table row and your club to its members with the trainer', () => {
    render(Results, {
      result: recommendations({
        trainer: { trainer_id: 123456789012, pace_per_day: 7_600_000 },
        total: 2,
        clubs,
        your_club: yourClub(),
      }),
    });
    const links = screen.getAllByRole('link', { name: 'See how you fit in Club 1' });
    // The top pick card, the top pick in the Your club comparison, and the table row.
    expect(links).toHaveLength(3);
    for (const link of links) {
      expect(link).toHaveAttribute('href', '/club/1?trainer=123456789012');
    }
    expect(screen.getByRole('link', { name: 'See how you fit in Home Club' })).toHaveAttribute(
      'href',
      '/club/99?trainer=123456789012',
    );
  });

  it('links without a trainer for a pace search', () => {
    render(Results, { result: recommendations({ total: 2, clubs }) });
    for (const link of screen.getAllByRole('link', { name: 'See members of Club 2' })) {
      expect(link).toHaveAttribute('href', '/club/2');
    }
  });
});

describe('Results states', () => {
  it('suggests how to widen an empty search', () => {
    render(Results, {
      result: recommendations({
        options: {
          balance_weight: 0.5,
          include_closed: false,
          include_full: false,
          join_styles: null,
          playstyles: null,
          has_discord: false,
          limit: 100,
        },
      }),
    });
    expect(screen.getByText('No clubs matched your search.')).toBeInTheDocument();
    expect(screen.getAllByRole('listitem')).toHaveLength(2);
  });

  it('dims stale results while a new search loads', () => {
    const { container } = render(Results, {
      result: recommendations({ total: 1, clubs: [club()] }),
      stale: true,
    });
    const section = container.querySelector('section');
    expect(section).toHaveClass('stale');
    expect(section).toHaveAttribute('aria-busy', 'true');
  });

  it('shows no notices for complete, current results', () => {
    render(Results, { result: recommendations() });
    expect(screen.queryByTestId('result-notice')).not.toBeInTheDocument();
  });

  it('says how many clubs are still loading', () => {
    render(Results, {
      result: recommendations({
        coverage: { considered: 340, shortlisted: 10, scored: 6, pending: 4 },
      }),
    });
    expect(screen.getByTestId('result-notice')).toHaveTextContent(
      '4 more clubs are still loading. Search again in a minute for complete results.',
    );
  });

  it("says when results use last month's data", () => {
    render(Results, {
      result: recommendations({ updated_at: '2026-03-02T15:00:00Z', month: '2026-02' }),
    });
    expect(screen.getByTestId('result-notice')).toHaveTextContent(
      "These results are based on last month's data (February 2026)",
    );
  });
});
