// MembersLink component tests. All IDs are made up.

import { fireEvent, render, screen } from '@testing-library/svelte';
import { afterEach, describe, expect, it } from 'vitest';

import MembersLink from './MembersLink.svelte';
import { rememberedSearch } from './route';

afterEach(() => {
  sessionStorage.clear();
});

describe('MembersLink', () => {
  it('invites the trainer to see how they fit, linking to the club page with them', () => {
    render(MembersLink, { circleId: 7, name: 'Example Club', trainerId: '123456789012' });
    const link = screen.getByRole('link', { name: 'See how you fit in Example Club' });
    expect(link).toHaveAttribute('href', '/club/7?trainer=123456789012');
    expect(link).not.toHaveAttribute('target');
    expect(link).toHaveTextContent('See how you fit →');
    expect(link).not.toHaveClass('compact');
  });

  it('offers to see the members for a pace search, linking to the club alone', () => {
    render(MembersLink, { circleId: 7, name: 'Example Club', trainerId: null });
    const link = screen.getByRole('link', { name: 'See members of Example Club' });
    expect(link).toHaveAttribute('href', '/club/7');
    expect(link).toHaveTextContent('See members →');
  });

  it('has a compact size for table rows', () => {
    render(MembersLink, { circleId: 7, name: 'Example Club', trainerId: null, compact: true });
    expect(screen.getByRole('link')).toHaveClass('compact');
  });

  it('remembers the search when followed', async () => {
    render(MembersLink, { circleId: 7, name: 'Example Club', trainerId: null });
    const link = screen.getByRole('link');
    link.addEventListener('click', (event) => event.preventDefault());
    await fireEvent.click(link);
    expect(rememberedSearch(sessionStorage)).toBe(window.location.href);
  });
});
