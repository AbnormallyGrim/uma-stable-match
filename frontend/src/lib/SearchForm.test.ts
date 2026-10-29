// Search form tests. All data is made up.

import { fireEvent, render, screen } from '@testing-library/svelte';
import { describe, expect, it, vi } from 'vitest';

import SearchForm from './SearchForm.svelte';
import { DEFAULT_SEARCH } from './search';

const TRAINER_ID_ERROR = 'Enter your trainer ID: 9 to 12 digits.';
const PACE_ERROR = 'Enter your daily fan gain in millions, such as 7.6.';

function setup(overrides: Partial<typeof DEFAULT_SEARCH> = {}, remember = false) {
  const onsearch = vi.fn();
  render(SearchForm, { initial: { ...DEFAULT_SEARCH, ...overrides }, remember, onsearch });
  return onsearch;
}

const submit = () => fireEvent.click(screen.getByRole('button', { name: 'Find clubs' }));

describe('SearchForm', () => {
  it('submits a valid trainer ID', async () => {
    const onsearch = setup();
    await fireEvent.input(screen.getByLabelText('Trainer ID', { selector: 'input#trainer-id' }), {
      target: { value: '123456789012' },
    });
    await submit();
    expect(onsearch).toHaveBeenCalledWith(
      {
        source: { trainerId: '123456789012' },
        balanceWeight: null,
        joinStyle: null,
        playstyle: null,
        hasDiscord: false,
        includeFull: false,
      },
      expect.objectContaining({ trainerId: '123456789012' }),
      false,
    );
  });

  it('shows an error and does not submit an invalid trainer ID', async () => {
    const onsearch = setup({ trainerId: '123' });
    await submit();
    expect(onsearch).not.toHaveBeenCalled();
    const input = screen.getByLabelText('Trainer ID', { selector: 'input#trainer-id' });
    expect(input).toHaveAttribute('aria-invalid', 'true');
    expect(input).toHaveAccessibleDescription(TRAINER_ID_ERROR);
  });

  it('switches to pace and validates it', async () => {
    const onsearch = setup();
    await fireEvent.click(screen.getByLabelText('Daily pace'));
    const input = screen.getByLabelText('Daily fan gain (millions)');
    await fireEvent.input(input, { target: { value: 'fast' } });
    await submit();
    expect(screen.getByText(PACE_ERROR)).toBeInTheDocument();

    await fireEvent.input(input, { target: { value: '7.6' } });
    await submit();
    expect(onsearch).toHaveBeenCalledWith(
      expect.objectContaining({ source: { pace: 7_600_000 } }),
      expect.objectContaining({ mode: 'pace' }),
      false,
    );
    expect(screen.queryByText(PACE_ERROR)).not.toBeInTheDocument();
  });

  it('only enables the strength slider with balance scoring on', async () => {
    const onsearch = setup({ trainerId: '123456789' });
    const slider = screen.getByLabelText(/Balance strength/);
    expect(slider).toBeDisabled();

    await fireEvent.click(screen.getByLabelText('Balance scoring'));
    expect(slider).toBeEnabled();
    await fireEvent.input(slider, { target: { value: '0.8' } });
    expect(screen.getByText('Balance strength: 0.80')).toBeInTheDocument();

    await submit();
    expect(onsearch.mock.lastCall?.[0]).toMatchObject({ balanceWeight: 0.8 });
  });

  it('passes one join style and the remember choice', async () => {
    const onsearch = setup({ trainerId: '123456789' }, true);
    expect(screen.getByLabelText('Remember my trainer ID on this device')).toBeChecked();
    const open = screen.getByRole('button', { name: 'Open' });
    const closed = screen.getByRole('button', { name: 'Closed' });
    expect(open).toHaveAttribute('aria-pressed', 'false');
    await fireEvent.click(open);
    await fireEvent.click(closed);
    expect(open).toHaveAttribute('aria-pressed', 'false');
    expect(closed).toHaveAttribute('aria-pressed', 'true');
    await submit();
    expect(onsearch.mock.lastCall?.[0]).toMatchObject({ joinStyle: 'closed' });
    expect(onsearch.mock.lastCall?.[2]).toBe(true);
  });

  it('clears the join style when it is picked again', async () => {
    const onsearch = setup({ trainerId: '123456789' });
    const approval = screen.getByRole('button', { name: 'Approval' });
    await fireEvent.click(approval);
    await fireEvent.click(approval);
    expect(approval).toHaveAttribute('aria-pressed', 'false');
    await submit();
    expect(onsearch.mock.lastCall?.[0]).toMatchObject({ joinStyle: null });
  });

  it('picks one playstyle and warns that filters narrow the search', async () => {
    const onsearch = setup({ trainerId: '123456789' });
    const playstyle = screen.getByRole('combobox', { name: /Playstyle/ });
    expect(playstyle).toHaveTextContent('Any playstyle');
    expect(screen.queryByTestId('filter-warning')).not.toBeInTheDocument();
    await fireEvent.click(playstyle);
    await fireEvent.click(screen.getByRole('option', { name: 'Log in Daily' }));
    expect(playstyle).toHaveTextContent('Log in Daily');
    expect(screen.queryByRole('listbox')).not.toBeInTheDocument();
    expect(screen.getByTestId('filter-warning')).toHaveTextContent('further from your pace');
    await submit();
    expect(onsearch.mock.lastCall?.[0]).toMatchObject({ playstyle: 12 });
  });

  it('passes Has Discord and explains it is a best guess', async () => {
    const onsearch = setup({ trainerId: '123456789' });
    expect(screen.queryByTestId('discord-warning')).not.toBeInTheDocument();
    await fireEvent.click(screen.getByRole('switch', { name: 'Has Discord' }));
    expect(screen.getByTestId('discord-warning')).toHaveTextContent(
      "Some clubs with a Discord won't show up",
    );
    await submit();
    expect(onsearch.mock.lastCall?.[0]).toMatchObject({ hasDiscord: true });
  });

  it('shows balance scoring and full clubs as switches', () => {
    setup();
    expect(screen.getByRole('switch', { name: 'Balance scoring' })).not.toBeChecked();
    expect(screen.getByRole('switch', { name: 'Include full clubs' })).not.toBeChecked();
  });

  it('passes include full', async () => {
    const onsearch = setup({ trainerId: '123456789' });
    await fireEvent.click(screen.getByLabelText('Include full clubs'));
    await submit();
    expect(onsearch.mock.lastCall?.[0]).toMatchObject({ includeFull: true });
  });

  it('starts from the initial search', () => {
    setup({ mode: 'pace', paceMillions: '5', balance: true, balanceWeight: 0.3 });
    expect(screen.getByLabelText('Daily fan gain (millions)')).toHaveValue('5');
    expect(screen.getByLabelText('Balance scoring')).toBeChecked();
    expect(screen.getByText('Balance strength: 0.30')).toBeInTheDocument();
  });

  it('disables the button while busy', () => {
    render(SearchForm, { initial: DEFAULT_SEARCH, remember: false, busy: true, onsearch: vi.fn() });
    expect(screen.getByRole('button', { name: 'Searching…' })).toBeDisabled();
  });
});
