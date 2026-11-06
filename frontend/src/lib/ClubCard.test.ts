// ClubCard component tests. All data is made up.

import { render, screen } from '@testing-library/svelte';
import { afterEach, describe, expect, it } from 'vitest';

import ClubCard from './ClubCard.svelte';
import { flags } from './flags.svelte';
import { club } from './test-data';

describe('ClubCard', () => {
  afterEach(() => {
    flags.applications = false;
  });

  it('shows the club and links to uma.moe in a new tab', () => {
    render(ClubCard, { club: club(), position: 1 });
    const link = screen.getByRole('link', { name: 'Example Club' });
    expect(link).toHaveAttribute('href', 'https://uma.moe/circles/1');
    expect(link).toHaveAttribute('target', '_blank');
    expect(link).toHaveAttribute('rel', 'noopener noreferrer');
    expect(screen.getByRole('article', { name: 'Example Club' })).toBeInTheDocument();
    expect(screen.getByRole('img', { name: 'Club tier A+' })).toBeInTheDocument();
  });

  it('shows rank, trend, spots and tags', () => {
    render(ClubCard, { club: club(), position: 4 });
    expect(screen.getByText(/Rank 111/)).toBeInTheDocument();
    expect(screen.getByText('▲16')).toBeInTheDocument();
    expect(screen.getByText('Up 16 places since last month')).toBeInTheDocument();
    expect(screen.getByText('5 open spots')).toBeInTheDocument();
    expect(screen.getByText('Open')).toBeInTheDocument();
    expect(screen.getByText('Laid-back')).toBeInTheDocument();
    expect(screen.getByText('Made-up description')).toBeInTheDocument();
  });

  it('shows how you compare, without fit', () => {
    render(ClubCard, { club: club(), position: 1 });
    expect(screen.getByText('9 of 26')).toBeInTheDocument();
    expect(screen.getByText('6.4M/day')).toBeInTheDocument();
    expect(screen.queryByText('Fit')).not.toBeInTheDocument();
    expect(screen.queryByText('Balance')).not.toBeInTheDocument();
  });

  it('shows balance details when the club has them', () => {
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
    });
    expect(screen.queryByText('Score')).not.toBeInTheDocument();
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
    });
    expect(screen.getByText('Club tier unknown')).toBeInTheDocument();
    expect(screen.getByText('new')).toBeInTheDocument();
    expect(screen.getByText('1 open spot')).toBeInTheDocument();
    expect(screen.queryByText('Made-up description')).not.toBeInTheDocument();
  });

  it('marks the top three picks', () => {
    const { container } = render(ClubCard, { club: club(), position: 2 });
    expect(container.querySelector('article')).toHaveClass('p2');
    expect(screen.getByLabelText('Pick 2')).toHaveTextContent('2');
  });

  it('tags clubs whose description mentions Discord', () => {
    render(ClubCard, { club: club({ has_discord: true }), position: 4 });
    expect(screen.getByText('Discord')).toHaveAttribute(
      'title',
      'The description mentions Discord',
    );
  });

  it('shows no Discord tag otherwise', () => {
    render(ClubCard, { club: club(), position: 4 });
    expect(screen.queryByText('Discord')).not.toBeInTheDocument();
  });

  it('hides Quick Apply while applications are off', () => {
    render(ClubCard, { club: club({ applications: 'discord' }), position: 1 });
    expect(screen.queryByText('Quick Apply')).not.toBeInTheDocument();
  });

  it('offers Quick Apply on clubs with the bot', () => {
    flags.applications = true;
    render(ClubCard, { club: club({ applications: 'discord' }), position: 1 });
    const link = screen.getByRole('link', { name: 'Quick Apply' });
    expect(link).toHaveAttribute('href', '/api/v1/discord/apply?club=1');
  });

  it('greys out Quick Apply, with a tooltip, on clubs without the bot', () => {
    flags.applications = true;
    render(ClubCard, { club: club(), position: 1 });
    const button = screen.getByRole('button', { name: /Quick Apply/ });
    expect(button).toHaveAttribute('aria-disabled', 'true');
    const tip = "This club doesn't have quick applications enabled";
    expect(button.parentElement).toHaveAttribute('data-tip', tip);
    expect(button).toHaveAccessibleDescription(tip);
  });
});
