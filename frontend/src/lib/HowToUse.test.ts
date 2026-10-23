// HowToUse component tests.

import { fireEvent, render, screen } from '@testing-library/svelte';
import { describe, expect, it } from 'vitest';

import HowToUse from './HowToUse.svelte';

describe('HowToUse', () => {
  it('starts closed and opens on request', async () => {
    const { container } = render(HowToUse);
    const details = container.querySelector('details')!;
    expect(details.open).toBe(false);
    await fireEvent.click(screen.getByText('How to use'));
    expect(details.open).toBe(true);
  });

  it('explains how to find the trainer ID and the pace', () => {
    render(HowToUse);
    const steps = screen.getAllByRole('listitem', { hidden: true });
    expect(steps).toHaveLength(5);
    expect(steps[0]).toHaveTextContent('Menu → Profile');
    expect(steps[0]).toHaveTextContent('9 to 12 digit');
    expect(steps[2]).toHaveTextContent('Daily pace');
    expect(steps[2]).toHaveTextContent('7.6 million a day');
  });

  it('links to uma.moe rankings in a new tab', () => {
    render(HowToUse);
    const link = screen.getByRole('link', { name: "uma.moe's rankings", hidden: true });
    expect(link).toHaveAttribute('href', 'https://uma.moe/rankings');
    expect(link).toHaveAttribute('target', '_blank');
    expect(link).toHaveAttribute('rel', 'noopener');
  });
});
