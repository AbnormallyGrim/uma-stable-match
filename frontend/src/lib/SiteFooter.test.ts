// SiteFooter component tests.

import { render, screen, within } from '@testing-library/svelte';
import { describe, expect, it } from 'vitest';

import SiteFooter from './SiteFooter.svelte';

describe('SiteFooter', () => {
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

  it('links to the issue tracker to report a problem', () => {
    render(SiteFooter);
    const nav = screen.getByRole('navigation', { name: 'Project links' });
    const links = within(nav).getAllByRole('link');
    expect(links.map((l) => l.textContent)).toEqual(['Report a problem']);
    expect(links[0]).toHaveAttribute(
      'href',
      'https://github.com/AbnormallyGrim/uma-stable-match/issues',
    );
    expect(links[0]).toHaveAttribute('rel', 'noopener noreferrer');
  });

  it('has the tips for switching clubs', () => {
    render(SiteFooter);
    const details = screen.getByText('Tips for switching clubs').closest('details')!;
    expect(within(details).getAllByRole('listitem')).toHaveLength(4);
    expect(details).toHaveTextContent('14-day rule');
  });
});
