// RunnerProgress component tests.

import { render } from '@testing-library/svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { PROGRESS_CAP } from './progress';
import RunnerProgress from './RunnerProgress.svelte';

function progressOf(container: HTMLElement): number {
  const track = container.querySelector<HTMLElement>('.track');
  return Number(track?.style.getPropertyValue('--progress'));
}

describe('RunnerProgress', () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it('is decorative, so screen readers skip it', () => {
    const { container } = render(RunnerProgress);
    expect(container.querySelector('.track')).toHaveAttribute('aria-hidden', 'true');
  });

  it('starts at the beginning of the track and moves along it over time', async () => {
    const { container } = render(RunnerProgress);
    expect(progressOf(container)).toBe(0);

    await vi.advanceTimersByTimeAsync(2000);
    const early = progressOf(container);
    expect(early).toBeGreaterThan(0);

    await vi.advanceTimersByTimeAsync(4000);
    expect(progressOf(container)).toBeGreaterThan(early);
  });

  it('never reaches the end while the search runs', async () => {
    const { container } = render(RunnerProgress);
    await vi.advanceTimersByTimeAsync(60 * 1000);
    expect(progressOf(container)).toBeLessThan(PROGRESS_CAP);

    await vi.advanceTimersByTimeAsync(10 * 60 * 1000);
    expect(progressOf(container)).toBeLessThanOrEqual(PROGRESS_CAP);
  });

  it('starts over for each search', async () => {
    const first = render(RunnerProgress);
    await vi.advanceTimersByTimeAsync(5000);
    expect(progressOf(first.container)).toBeGreaterThan(0.3);
    first.unmount();

    const second = render(RunnerProgress);
    expect(progressOf(second.container)).toBe(0);
  });

  it('stops animating once it is removed', async () => {
    const { unmount } = render(RunnerProgress);
    await vi.advanceTimersByTimeAsync(100);
    unmount();
    expect(vi.getTimerCount()).toBe(0);
  });
});
