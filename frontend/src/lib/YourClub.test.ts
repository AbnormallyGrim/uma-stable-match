// YourClub component tests. All data is made up.

import { render, screen, within } from '@testing-library/svelte';
import { describe, expect, it } from 'vitest';

import { club, yourClub } from './test-data';
import YourClub from './YourClub.svelte';

const topPick = club({ name: 'Best Fit Club', your_place: 12, club_size: 28 });

function setup(overrides: Partial<Parameters<typeof render<typeof YourClub>>[1]> = {}) {
  return render(YourClub, {
    own: yourClub(),
    topPick,
    pace: 7_600_000,
    count: 100,
    ...overrides,
  });
}

describe('YourClub', () => {
  it('names the club and links to its uma.moe page', () => {
    setup();
    const link = screen.getByRole('link', { name: 'Home Club' });
    expect(link).toHaveAttribute('href', 'https://uma.moe/circles/99');
    expect(link).toHaveAttribute('target', '_blank');
    expect(screen.getByText('Full')).toBeInTheDocument();
    expect(screen.getByText('Closed')).toBeInTheDocument();
  });

  it('says where the club would rank and compares its median with your pace', () => {
    setup();
    const card = screen.getByTestId('your-club');
    expect(card).toHaveTextContent('Your club would be #37 in your results.');
    expect(card).toHaveTextContent('Its typical member gains 11.2M/day, against your 7.6M/day.');
  });

  it('says when the club fits better than every result', () => {
    setup({ own: yourClub({ result_rank: 1 }) });
    expect(screen.getByTestId('your-club')).toHaveTextContent(
      'Your club fits you better than any of your results.',
    );
  });

  it('says when the club ranks below the results shown', () => {
    setup({ own: yourClub({ result_rank: 101 }) });
    expect(screen.getByTestId('your-club')).toHaveTextContent(
      'Your club would rank below your top 100.',
    );
  });

  it('compares the club with the top pick', () => {
    setup();
    const own = screen.getByTestId('your-club-own');
    expect(own).toHaveTextContent('Median 11.2M/day');
    expect(own).toHaveTextContent('Vs your pace +3.6M/day');
    expect(own).toHaveTextContent("You'd be 24 of 30");
    const top = screen.getByTestId('your-club-top');
    expect(within(top).getByRole('link', { name: 'Best Fit Club' })).toBeInTheDocument();
    expect(top).toHaveTextContent('Up 16 places since last month');
    expect(top).toHaveTextContent('5 open spots');
    expect(top).toHaveTextContent('Median 6.4M/day');
    expect(top).toHaveTextContent('Vs your pace −1.2M/day');
    expect(top).toHaveTextContent("You'd be 12 of 28");
    expect(screen.queryByText('Fit')).not.toBeInTheDocument();
  });

  it('shows balance details when a club has them', () => {
    setup({
      topPick: club({
        balance: {
          score: 88,
          gini: 0.1,
          passengers: 2,
          counted: 20,
          bottom_quarter_per_day: 3_000_000,
          top3_share: 0.3,
        },
      }),
    });
    const top = screen.getByTestId('your-club-top');
    expect(top).toHaveTextContent('Balance 88/100');
    expect(top).toHaveTextContent('Passengers 2 of 20');
    expect(top).toHaveTextContent('Top 3 share 30%');
    expect(screen.getByTestId('your-club-own')).not.toHaveTextContent('Balance');
  });

  it('shows only the club without a top pick', () => {
    setup({ topPick: null });
    expect(screen.queryByText('Top pick')).not.toBeInTheDocument();
    expect(screen.queryByTestId('your-club-top')).not.toBeInTheDocument();
  });
});
