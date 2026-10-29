// Product tour tests: the overlay component, and remembering that the tour was seen.

import { fireEvent, render, screen } from '@testing-library/svelte';
import { describe, expect, it, vi } from 'vitest';

import { hasSeenTour, markTourSeen, TOUR_KEY, tourSteps } from './tour';
import Tour from './Tour.svelte';

const steps = [
  { title: 'First', body: 'Start here.' },
  { target: 'thing', title: 'Second', body: 'Look at this.' },
  { title: 'Third', body: 'All done.' },
];

function setup() {
  const onclose = vi.fn();
  render(Tour, { steps, onclose });
  return onclose;
}

describe('Tour', () => {
  it('opens on the first step as a labelled dialog with Next focused', async () => {
    setup();
    const dialog = screen.getByRole('dialog', { name: 'First' });
    expect(dialog).toHaveAttribute('aria-modal', 'true');
    expect(dialog).toHaveAccessibleDescription('Start here.');
    expect(screen.getByText('Step 1 of 3')).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Back' })).not.toBeInTheDocument();
    await vi.waitFor(() => expect(screen.getByRole('button', { name: 'Next' })).toHaveFocus());
  });

  it('steps forward and back', async () => {
    setup();
    await fireEvent.click(screen.getByRole('button', { name: 'Next' }));
    expect(screen.getByRole('dialog', { name: 'Second' })).toBeInTheDocument();
    await fireEvent.click(screen.getByRole('button', { name: 'Back' }));
    expect(screen.getByRole('dialog', { name: 'First' })).toBeInTheDocument();
  });

  it('finishes with Done on the last step', async () => {
    const onclose = setup();
    await fireEvent.click(screen.getByRole('button', { name: 'Next' }));
    await fireEvent.click(screen.getByRole('button', { name: 'Next' }));
    expect(screen.queryByRole('button', { name: 'Skip tour' })).not.toBeInTheDocument();
    await fireEvent.click(screen.getByRole('button', { name: 'Done' }));
    expect(onclose).toHaveBeenCalledOnce();
  });

  it('can be skipped with the button or Escape', async () => {
    const onclose = setup();
    await fireEvent.click(screen.getByRole('button', { name: 'Skip tour' }));
    expect(onclose).toHaveBeenCalledOnce();
    await fireEvent.keyDown(window, { key: 'Escape' });
    expect(onclose).toHaveBeenCalledTimes(2);
  });

  it('steps with the arrow keys', async () => {
    setup();
    await fireEvent.keyDown(window, { key: 'ArrowRight' });
    expect(screen.getByRole('dialog', { name: 'Second' })).toBeInTheDocument();
    await fireEvent.keyDown(window, { key: 'ArrowLeft' });
    expect(screen.getByRole('dialog', { name: 'First' })).toBeInTheDocument();
  });
});

describe('tour storage', () => {
  it('remembers that the tour was seen', () => {
    localStorage.clear();
    expect(hasSeenTour(localStorage)).toBe(false);
    markTourSeen(localStorage);
    expect(localStorage.getItem(TOUR_KEY)).toBe('1');
    expect(hasSeenTour(localStorage)).toBe(true);
    localStorage.clear();
  });

  it('counts blocked storage as seen, so the tour never repeats on every visit', () => {
    const blocked = {
      getItem: () => {
        throw new Error('blocked');
      },
      setItem: () => {
        throw new Error('blocked');
      },
    } as unknown as Storage;
    expect(hasSeenTour(blocked)).toBe(true);
    expect(hasSeenTour(undefined)).toBe(true);
    expect(() => markTourSeen(blocked)).not.toThrow();
  });

  it('starts with a welcome and ends on the help button', () => {
    expect(tourSteps()[0]!.target).toBeUndefined();
    expect(tourSteps().at(-1)!.target).toBe('help');
  });

  it('only highlights parts of the page that exist', () => {
    const targets = tourSteps().flatMap((step) => (step.target ? [step.target] : []));
    expect(targets).toEqual([
      'search-by',
      'search-field',
      'balance',
      'filters',
      'find',
      'how-to',
      'theme',
      'help',
    ]);
  });
});
