// ClubCard component tests. All data is made up.

import { render, screen } from '@testing-library/svelte';
import { describe, expect, it } from 'vitest';

import ClubCard from './ClubCard.svelte';
import { club } from './test-data';

describe('ClubCard', () => {
  it('shows the club and links to uma.moe in a new tab', () => {
    render(ClubCard, { club: club(), position: 1, balanceOn: false });
    const link = screen.getByRole('link', { name: 'Example Club' });
    expect(link).toHaveAttribute('href', 'https://uma.moe/circles/1');
    expect(link).toHaveAttribute('target', '_blank');
    expect(link).toHaveAttribute('rel', 'noopener noreferrer');
    expect(screen.getByRole('article', { name: 'Example Club' })).toBeInTheDocument();
    expect(screen.getByRole('img', { name: 'Club tier A+' })).toBeInTheDocument();
  });

  it('shows rank, trend, spots and tags', () => {
    render(ClubCard, { club: club(), position: 4, balanceOn: false });
    expect(screen.getByText(/Rank 111/)).toBeInTheDocument();
    expect(screen.getByText('▲16')).toBeInTheDocument();
    expect(screen.getByText('Up 16 places since last month')).toBeInTheDocument();
    expect(screen.getByText('5 open spots')).toBeInTheDocument();
    expect(screen.getByText('Open')).toBeInTheDocument();
    expect(screen.getByText('Laid-back')).toBeInTheDocument();
    expect(screen.getByText('Made-up description')).toBeInTheDocument();
  });

  it('shows how you compare, with fit when balance is off', () => {
    render(ClubCard, { club: club(), position: 1, balanceOn: false });
    expect(screen.getByText('9 of 26')).toBeInTheDocument();
    expect(screen.getByText('6.4M/day')).toBeInTheDocument();
    expect(screen.getByText('Fit')).toBeInTheDocument();
    expect(screen.getByText('0.056')).toBeInTheDocument();
    expect(screen.queryByText('Balance')).not.toBeInTheDocument();
  });

  it('shows score and balance details when balance is on', () => {
    render(ClubCard, {
      club: club({
        balance: {
          score: 90,
          gini: 0.12,
          passengers: 1,
          counted: 24,
          bottom_quarter_per_day: 3_100_000,
          top3_share: 0.21,
        },
      }),
      position: 1,
      balanceOn: true,
    });
    expect(screen.getByText('Score')).toBeInTheDocument();
    expect(screen.getByText('0.106')).toBeInTheDocument();
    expect(screen.getByText('90/100')).toBeInTheDocument();
    expect(screen.getByText('1 of 24')).toBeInTheDocument();
    expect(screen.getByText('21%')).toBeInTheDocument();
  });

  it('handles missing and edge-case details', () => {
    render(ClubCard, {
      club: club({
        tier: null,
        join_style: null,
        policy: 99,
        monthly_rank: 50,
        last_month_rank: null,
        open_spots: 1,
        comment: null,
      }),
      position: 2,
      balanceOn: false,
    });
    expect(screen.getByText('Club tier unknown')).toBeInTheDocument();
    expect(screen.getByText('new')).toBeInTheDocument();
    expect(screen.getByText('1 open spot')).toBeInTheDocument();
    expect(screen.queryByText('Made-up description')).not.toBeInTheDocument();
  });

  it('marks the top three picks', () => {
    const { container } = render(ClubCard, { club: club(), position: 2, balanceOn: false });
    expect(container.querySelector('article')).toHaveClass('p2');
    expect(screen.getByLabelText('Pick 2')).toHaveTextContent('2');
  });

  it('tags clubs whose description mentions Discord', () => {
    render(ClubCard, { club: club({ has_discord: true }), position: 4, balanceOn: false });
    expect(screen.getByText('Discord')).toHaveAttribute(
      'title',
      'The description mentions Discord',
    );
  });

  it('shows no Discord tag otherwise', () => {
    render(ClubCard, { club: club(), position: 4, balanceOn: false });
    expect(screen.queryByText('Discord')).not.toBeInTheDocument();
  });
});
