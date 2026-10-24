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
    render(Results, { result: recommendations({ total: 42, clubs }) });
    const summary = screen.getByTestId('result-summary');
    expect(within(summary).getByText('7.6M/day')).toBeInTheDocument();
    expect(within(summary).getByText('42')).toBeInTheDocument();
    expect(within(summary).getByText('2')).toBeInTheDocument();
    expect(within(summary).getByText('Top 35%')).toBeInTheDocument();
  });

  it("doesn't repeat the data age or the balance setting", () => {
    render(Results, {
      result: recommendations({
        options: { balance_weight: 0.5, include_closed: false, limit: 50 },
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
    expect(within(items[0]!).getByRole('link')).toHaveTextContent('Club 1');
    expect(within(items[9]!).getByRole('link')).toHaveTextContent('Club 10');
    expect(within(picks).queryByText('Club 11')).not.toBeInTheDocument();
    expect(within(screen.getByRole('table')).getAllByRole('link')).toHaveLength(12);
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

  it('says when nothing matched', () => {
    render(Results, { result: recommendations() });
    expect(screen.getByText('No clubs matched your search.')).toBeInTheDocument();
  });
});

describe('Results states', () => {
  it('suggests how to widen an empty search', () => {
    render(Results, {
      result: recommendations({
        options: { balance_weight: 0.5, include_closed: false, limit: 100 },
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
      result: recommendations({ coverage: { shortlisted: 10, scored: 6, pending: 4 } }),
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
