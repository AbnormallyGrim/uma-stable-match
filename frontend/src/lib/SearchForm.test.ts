import { fireEvent, render, screen } from '@testing-library/svelte';
import { describe, expect, it, vi } from 'vitest';

import SearchForm from './SearchForm.svelte';
import { DEFAULT_SEARCH, PACE_ERROR, TRAINER_ID_ERROR } from './search';

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
      { source: { trainerId: '123456789012' }, balanceWeight: null, includeClosed: false },
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

  it('passes include closed and remember choices', async () => {
    const onsearch = setup({ trainerId: '123456789' }, true);
    expect(screen.getByLabelText('Remember my trainer ID on this device')).toBeChecked();
    await fireEvent.click(screen.getByLabelText('Include closed clubs'));
    await submit();
    expect(onsearch.mock.lastCall?.[0]).toMatchObject({ includeClosed: true });
    expect(onsearch.mock.lastCall?.[2]).toBe(true);
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
