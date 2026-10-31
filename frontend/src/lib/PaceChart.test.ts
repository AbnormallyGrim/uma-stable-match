// PaceChart component tests. All data is made up.

import { fireEvent, render, screen, within } from '@testing-library/svelte';
import { describe, expect, it } from 'vitest';

import type { ClubDetails } from './api';
import PaceChart from './PaceChart.svelte';

const M = 1_000_000;

// Cumulative fans from 100M, gaining `pace` a day for `days` days.
const fans = (pace: number, days = 6) =>
  Array.from({ length: 8 }, (_, d) => (d < days ? 100 * M + pace * d : 0));

function details(withTrainer = true): Pick<ClubDetails, 'members' | 'trainer' | 'month'> {
  return {
    month: '2026-03',
    members: [4, 6, 8].map((p, i) => ({
      name: `Made-up Trainer ${'ABC'[i]}`,
      pace_per_day: p * M,
      daily_fans: fans(p * M),
    })),
    trainer: withTrainer
      ? {
          trainer_id: 123456789012,
          pace_per_day: 7 * M,
          daily_fans: fans(7 * M),
          in_club: false,
          your_place: 2,
          club_size: 4,
        }
      : null,
  };
}

const slider = () => screen.getByRole('slider');

describe('PaceChart', () => {
  it('draws every member, the median and the trainer, with a legend', () => {
    const { container } = render(PaceChart, { details: details() });
    expect(container.querySelectorAll('path.member')).toHaveLength(3);
    expect(container.querySelector('path.median')).toHaveAttribute('d');
    expect(container.querySelector('path.yours')).toHaveAttribute('d');
    const legend = within(screen.getByRole('list'));
    expect(legend.getAllByRole('listitem').map((li) => li.textContent?.trim())).toEqual([
      'Members',
      'Club median',
      'You',
    ]);
  });

  it('leaves the trainer out when there is none', () => {
    const { container } = render(PaceChart, { details: details(false) });
    expect(container.querySelector('path.yours')).toBeNull();
    expect(screen.queryByText('You')).not.toBeInTheDocument();
  });

  it('reads each day with the arrow keys, daily gain first', async () => {
    render(PaceChart, { details: details() });
    expect(slider()).toHaveAttribute('aria-valuemin', '2');
    expect(slider()).toHaveAttribute('aria-valuemax', '6');
    await fireEvent.keyDown(slider(), { key: 'Home' });
    expect(slider()).toHaveAttribute('aria-valuenow', '2');
    expect(slider()).toHaveAttribute(
      'aria-valuetext',
      'Day 2, You 7.0M/day, Club median 6.0M/day, Members 4.0M/day to 8.0M/day',
    );
    await fireEvent.keyDown(slider(), { key: 'ArrowRight' });
    expect(slider()).toHaveAttribute('aria-valuenow', '3');
    await fireEvent.keyDown(slider(), { key: 'End' });
    await fireEvent.keyDown(slider(), { key: 'ArrowRight' });
    expect(slider()).toHaveAttribute('aria-valuenow', '6');
  });

  it('switches to fans this month', async () => {
    render(PaceChart, { details: details() });
    await fireEvent.click(screen.getByRole('radio', { name: 'Fans this month' }));
    expect(slider()).toHaveAttribute('aria-valuemin', '1');
    await fireEvent.keyDown(slider(), { key: 'End' });
    expect(slider()).toHaveAttribute(
      'aria-valuetext',
      'Day 6, You 35.0M, Club median 30.0M, Members 20.0M to 40.0M',
    );
  });

  it('highlights the chosen member, naming them in the legend and readout', async () => {
    const { container } = render(PaceChart, { details: details(), highlight: 0 });
    expect(container.querySelector('path.chosen')).toHaveAttribute('d');
    expect(container.querySelector('circle.end.chosen')).toBeInTheDocument();
    expect(within(screen.getByRole('list')).getByText('Made-up Trainer A')).toBeInTheDocument();
    await fireEvent.keyDown(slider(), { key: 'Home' });
    expect(slider().getAttribute('aria-valuetext')).toMatch(
      /^Day 2, Made-up Trainer A 4\.0M\/day, /,
    );
  });

  it('names an unknown member in the legend', () => {
    const unnamed = details();
    unnamed.members[1] = { ...unnamed.members[1]!, name: null };
    render(PaceChart, { details: unnamed, highlight: 1 });
    expect(within(screen.getByRole('list')).getByText('Unknown trainer')).toBeInTheDocument();
  });

  it('leaves out the day still under way in the current month', async () => {
    // 19 March 2026 is in the game month of the made-up data, whose day 6 is today.
    const now = new Date('2026-03-19T09:00:00Z');
    const { container } = render(PaceChart, { details: details(), now });
    expect(slider()).toHaveAttribute('aria-valuemax', '5');
    await fireEvent.keyDown(slider(), { key: 'End' });
    expect(slider().getAttribute('aria-valuetext')).toMatch(/^Day 5, /);
    const dot = container.querySelector('circle.end');
    const last = container.querySelectorAll('.grid text');
    expect(dot).toBeInTheDocument();
    expect(Number(dot!.getAttribute('cx'))).toBeCloseTo(
      Number(last[last.length - 1]!.getAttribute('x')),
    );
  });

  it('treats data without a month as the current month', () => {
    render(PaceChart, { details: { ...details(), month: null } });
    expect(slider()).toHaveAttribute('aria-valuemax', '5');
  });

  it('has no table of numbers', () => {
    render(PaceChart, { details: details() });
    expect(screen.queryByRole('table')).not.toBeInTheDocument();
  });

  it('says when there are too few days for a chart', () => {
    render(PaceChart, {
      details: {
        month: '2026-03',
        members: [{ name: null, pace_per_day: null, daily_fans: fans(M, 1) }],
        trainer: null,
      },
    });
    expect(screen.getByText('There are too few days of data for a chart yet.')).toBeVisible();
    expect(screen.queryByRole('slider')).not.toBeInTheDocument();
  });
});
