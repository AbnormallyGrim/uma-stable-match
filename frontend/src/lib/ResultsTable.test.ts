// ResultsTable component tests. All data is made up.

import { fireEvent, render, screen, within } from '@testing-library/svelte';
import { describe, expect, it } from 'vitest';

import ResultsTable from './ResultsTable.svelte';
import { club } from './test-data';

const clubs = [
  club({
    circle_id: 1,
    name: 'Bravo',
    url: 'https://uma.moe/circles/1',
    tier: 'A',
    monthly_rank: 120,
    open_spots: 2,
    median_per_day: 6_000_000,
    fit: 0.05,
    score: 0.2,
  }),
  club({
    circle_id: 2,
    name: 'Alpha',
    url: 'https://uma.moe/circles/2',
    tier: 'S',
    monthly_rank: 90,
    open_spots: 5,
    median_per_day: 7_000_000,
    join_style: 'closed',
    fit: 0.08,
    score: 0.1,
  }),
  club({
    circle_id: 3,
    name: 'Charlie',
    url: 'https://uma.moe/circles/3',
    tier: null,
    monthly_rank: null,
    open_spots: null,
    median_per_day: 5_000_000,
    fit: 0.12,
    score: 0.3,
  }),
];

function setup(props: Partial<{ balanceOn: boolean; includeClosed: boolean }> = {}) {
  render(ResultsTable, { clubs, balanceOn: false, includeClosed: true, ...props });
  const table = screen.getByRole('table');
  const names = () =>
    within(table)
      .getAllByRole('link')
      .map((link) => link.textContent);
  const header = (name: string) =>
    within(table).getByRole('button', { name: new RegExp(`^${name}`) });
  return { table, names, header };
}

describe('ResultsTable', () => {
  it('lists clubs in pick order with links', () => {
    const { names, table } = setup({ includeClosed: true });
    expect(names()).toEqual(['Bravo', 'Charlie']);
    expect(within(table).getByRole('link', { name: 'Bravo' })).toHaveAttribute(
      'href',
      'https://uma.moe/circles/1',
    );
    expect(within(table).getAllByRole('columnheader')[0]).toHaveAttribute('aria-sort', 'ascending');
  });

  it('sorts by a column and toggles direction', async () => {
    const { names, header, table } = setup({ includeClosed: false });
    await fireEvent.click(header('Club'));
    expect(names()).toEqual(['Alpha', 'Bravo', 'Charlie']);
    const clubHeader = within(table).getAllByRole('columnheader')[2];
    expect(clubHeader).toHaveAttribute('aria-sort', 'ascending');

    await fireEvent.click(header('Club'));
    expect(names()).toEqual(['Charlie', 'Bravo', 'Alpha']);
    expect(clubHeader).toHaveAttribute('aria-sort', 'descending');
  });

  it('starts numeric columns in their most useful direction and puts missing values last', async () => {
    const { names, header } = setup({ includeClosed: false });
    await fireEvent.click(header('Open spots'));
    expect(names()).toEqual(['Alpha', 'Bravo', 'Charlie']);
    await fireEvent.click(header('Rank'));
    expect(names()).toEqual(['Alpha', 'Bravo', 'Charlie']);
    await fireEvent.click(header('Rank'));
    expect(names()).toEqual(['Bravo', 'Alpha', 'Charlie']);
    await fireEvent.click(header('Tier'));
    expect(names()).toEqual(['Alpha', 'Bravo', 'Charlie']);
  });

  it('searches by name', async () => {
    const { names } = setup({ includeClosed: false });
    await fireEvent.input(screen.getByLabelText('Search clubs by name'), {
      target: { value: 'ar' },
    });
    expect(names()).toEqual(['Charlie']);
    expect(screen.getByText(/Showing 1 of 3 clubs/)).toBeInTheDocument();

    await fireEvent.input(screen.getByLabelText('Search clubs by name'), {
      target: { value: 'zzz' },
    });
    expect(screen.getByText('No clubs match.')).toBeInTheDocument();
  });

  it('hides closed clubs only when they were included', async () => {
    const { names } = setup({ includeClosed: true });
    const toggle = screen.getByLabelText('Hide closed clubs');
    expect(toggle).toBeChecked();
    expect(names()).not.toContain('Alpha');
    await fireEvent.click(toggle);
    expect(names()).toContain('Alpha');
  });

  it('has no hide-closed toggle when closed clubs were not requested', () => {
    setup({ includeClosed: false });
    expect(screen.queryByLabelText('Hide closed clubs')).not.toBeInTheDocument();
  });

  it('shows balance columns and score only with balance on', () => {
    const { table } = setup({ includeClosed: false, balanceOn: false });
    const headers = () =>
      within(table)
        .getAllByRole('columnheader')
        .map((h) => h.textContent);
    expect(headers().join(' ')).not.toMatch(/Balance|Passengers|Top 3 share|Score/);
    expect(headers().join(' ')).toMatch(/Fit/);
  });

  it('shows balance details with balance on', () => {
    const balanced = [
      club({
        balance: {
          score: 88,
          gini: 0.1,
          passengers: 2,
          counted: 20,
          bottom_quarter_per_day: 3_000_000,
          top3_share: 0.3,
        },
      }),
    ];
    render(ResultsTable, { clubs: balanced, balanceOn: true, includeClosed: false });
    const row = screen.getAllByRole('row')[1]!;
    expect(within(row).getByText('88/100')).toBeInTheDocument();
    expect(within(row).getByText('2 of 20')).toBeInTheDocument();
    expect(within(row).getByText('30%')).toBeInTheDocument();
    expect(within(row).getByText('0.106')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /^Score/ })).toBeInTheDocument();
  });

  it('can be scrolled with the keyboard', () => {
    setup();
    expect(screen.getByRole('region', { name: 'All results' })).toHaveAttribute('tabindex', '0');
  });
});
