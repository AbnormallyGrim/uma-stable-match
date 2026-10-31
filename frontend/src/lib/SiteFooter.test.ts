// SiteFooter component tests.

import { fireEvent, render, screen, within } from '@testing-library/svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';

import SiteFooter from './SiteFooter.svelte';

describe('SiteFooter', () => {
  afterEach(() => vi.restoreAllMocks());

  it('is the page footer landmark with the disclaimer', () => {
    render(SiteFooter);
    const footer = screen.getByRole('contentinfo');
    expect(footer).toHaveTextContent('unofficial, non-commercial fan tool');
    expect(footer).toHaveTextContent(
      "It isn't affiliated with, endorsed by or sponsored by uma.moe or Cygames, Inc.",
    );
    expect(footer).toHaveTextContent('The club tier icons are © Cygames, Inc.');
  });

  it('credits uma.moe and links Cygames’ guidelines in new tabs', () => {
    render(SiteFooter);
    for (const [name, href] of [
      ['uma.moe', 'https://uma.moe'],
      ['Fan-Created Content Guidelines', 'https://umamusume.com/fan-createdguide'],
    ]) {
      const link = screen.getByRole('link', { name });
      expect(link).toHaveAttribute('href', href);
      expect(link).toHaveAttribute('target', '_blank');
      expect(link).toHaveAttribute('rel', 'noopener noreferrer');
    }
  });

  it('links to the wishlist page, except on the wishlist page', () => {
    const { unmount } = render(SiteFooter);
    expect(screen.getByRole('link', { name: 'Wishlist' })).toHaveAttribute('href', '/wishlist');
    expect(screen.getByText('Tips for switching clubs')).toBeInTheDocument();
    unmount();
    window.history.pushState({}, '', '/wishlist');
    try {
      render(SiteFooter);
      expect(screen.queryByRole('link', { name: 'Wishlist' })).not.toBeInTheDocument();
      expect(screen.queryByText('Tips for switching clubs')).not.toBeInTheDocument();
    } finally {
      window.history.pushState({}, '', '/');
    }
  });

  it('opens the issue tracker and Ko-fi in a new tab from buttons', async () => {
    const open = vi.spyOn(window, 'open').mockReturnValue(null);
    render(SiteFooter);
    const nav = screen.getByRole('navigation', { name: 'Project links' });
    const buttons = within(nav).getAllByRole('button');
    expect(buttons).toHaveLength(2);
    expect(buttons[0]).toHaveAccessibleName('Report a problem');
    expect(buttons[1]).toHaveAccessibleName('Buy me a carrot');
    await fireEvent.click(buttons[0]!);
    await fireEvent.click(buttons[1]!);
    expect(open).toHaveBeenNthCalledWith(
      1,
      'https://github.com/AbnormallyGrim/uma-stable-match/issues',
      '_blank',
      'noopener,noreferrer',
    );
    expect(open).toHaveBeenNthCalledWith(
      2,
      'https://ko-fi.com/L5P6299MV1',
      '_blank',
      'noopener,noreferrer',
    );
  });

  it('shows the Free carats button only on 1 April', async () => {
    const open = vi.spyOn(window, 'open').mockReturnValue(null);
    const nav = () => screen.getByRole('navigation', { name: 'Project links' });
    for (const today of [new Date(2026, 2, 31), new Date(2026, 3, 2), new Date(2026, 9, 28)]) {
      const { unmount } = render(SiteFooter, { today });
      expect(within(nav()).queryByRole('button', { name: 'Free carats' })).toBeNull();
      unmount();
    }
    render(SiteFooter, { today: new Date(2026, 3, 1, 23, 59) });
    const buttons = within(nav()).getAllByRole('button');
    expect(buttons[1]).toHaveAccessibleName('Buy me a carrot');
    await fireEvent.click(within(nav()).getByRole('button', { name: 'Free carats' }));
    expect(open).toHaveBeenCalledWith(
      'https://youtu.be/dQw4w9WgXcQ',
      '_blank',
      'noopener,noreferrer',
    );
  });

  it('says tips only cover hosting', () => {
    render(SiteFooter);
    expect(screen.getByRole('contentinfo')).toHaveTextContent(
      'Optional tips help cover hosting. Every feature stays free.',
    );
  });

  it('has the tips for switching clubs', () => {
    render(SiteFooter);
    const details = screen.getByText('Tips for switching clubs').closest('details')!;
    expect(within(details).getAllByRole('listitem')).toHaveLength(4);
    expect(details).toHaveTextContent('14-day rule');
  });
});
