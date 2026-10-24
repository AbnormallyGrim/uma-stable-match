// YourClub component tests. All data is made up.

import { render, screen, within } from '@testing-library/svelte';
import { describe, expect, it } from 'vitest';

import { club, yourClub } from './test-data';
import YourClub from './YourClub.svelte';

const topPick = club({ name: 'Best Fit Club', your_place: 12, club_size: 28, fit: 0.0562 });

function setup(overrides: Partial<Parameters<typeof render<typeof YourClub>>[1]> = {}) {
  return render(YourClub, {
    own: yourClub(),
    topPick,
    pace: 7_600_000,
    count: 100,
    balanceOn: false,
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
    const rows = within(screen.getByRole('table')).getAllByRole('row');
    expect(rows[0]).toHaveTextContent('Your clubTop pick');
    expect(rows[1]).toHaveTextContent('Median11.2M/day6.4M/day');
    expect(rows[2]).toHaveTextContent("You'd be24 of 3012 of 28");
    expect(rows[3]).toHaveTextContent('Fit0.4320.056');
    expect(screen.getByText(/Top pick: Best Fit Club/)).toBeInTheDocument();
  });

  it('shows the score with balance on', () => {
    setup({ balanceOn: true });
    const rows = within(screen.getByRole('table')).getAllByRole('row');
    expect(rows[3]).toHaveTextContent('Score0.4820.106');
  });

  it('shows only the club without a top pick', () => {
    setup({ topPick: null });
    expect(screen.queryByText('Top pick')).not.toBeInTheDocument();
    expect(screen.queryByText(/Top pick:/)).not.toBeInTheDocument();
  });
});
