import { fireEvent, render, screen } from '@testing-library/svelte';
import { describe, expect, it, vi } from 'vitest';

import type { SearchPhase } from './runner';
import SearchStatus from './SearchStatus.svelte';

function setup(status: SearchPhase) {
  const onretry = vi.fn();
  render(SearchStatus, { status, onretry });
  return onretry;
}

describe('SearchStatus', () => {
  it('shows nothing when idle or done', () => {
    for (const phase of ['idle', 'done'] as const) {
      const { container, unmount } = render(SearchStatus, { status: { phase }, onretry: vi.fn() });
      expect(container.textContent?.trim()).toBe('');
      expect(container.querySelector('p, div, button')).toBeNull();
      unmount();
    }
  });

  it('shows loading', () => {
    setup({ phase: 'loading' });
    expect(screen.getByRole('status')).toHaveTextContent('Finding clubs…');
  });

  it('counts down while the first scan runs and can retry now', async () => {
    const onretry = setup({ phase: 'waiting-for-scan', retryIn: 5 });
    expect(screen.getByText('Club data is still loading')).toBeInTheDocument();
    expect(screen.getByText(/Trying again in 5 seconds/)).toBeInTheDocument();
    await fireEvent.click(screen.getByRole('button', { name: 'Try now' }));
    expect(onretry).toHaveBeenCalledOnce();
  });

  it('disables retry until a rate limit runs out', () => {
    setup({ phase: 'rate-limited', retryIn: 1 });
    expect(screen.getByText('Please wait 1 second before searching again.')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Try again' })).toBeDisabled();
  });

  it('enables retry once a rate limit has run out', () => {
    setup({ phase: 'rate-limited', retryIn: 0 });
    expect(screen.getByText('You can search again now.')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Try again' })).toBeEnabled();
  });

  it('shows errors with a retry button only when retrying can help', async () => {
    const onretry = setup({
      phase: 'error',
      message: { title: "Couldn't reach uma.moe", detail: 'Try again later.', canRetry: true },
    });
    expect(screen.getByRole('alert')).toHaveTextContent("Couldn't reach uma.moe");
    await fireEvent.click(screen.getByRole('button', { name: 'Try again' }));
    expect(onretry).toHaveBeenCalledOnce();
  });

  it('hides the retry button when retrying cannot help', () => {
    setup({
      phase: 'error',
      message: { title: 'Not enough fan data yet', detail: 'Wait a few days.', canRetry: false },
    });
    expect(screen.queryByRole('button')).not.toBeInTheDocument();
  });
});
