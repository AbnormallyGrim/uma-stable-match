// ClubTags component tests. All data is made up.

import { render, screen } from '@testing-library/svelte';
import { describe, expect, it } from 'vitest';

import ClubTags from './ClubTags.svelte';
import { language } from './language.svelte';
import { club } from './test-data';

describe('ClubTags', () => {
  it('shows a Quick apply badge for clubs that take applications there', () => {
    render(ClubTags, { club: club({ applications: 'discord' }) });
    const badge = screen.getByText('Quick apply');
    expect(badge).toHaveAttribute(
      'title',
      'You can apply to this club right from Uma Stable Match',
    );
  });

  it('shows no badge otherwise', () => {
    render(ClubTags, { club: club() });
    expect(screen.queryByText('Quick apply')).not.toBeInTheDocument();
  });

  it('keeps the badge apart from the description mentioning Discord', () => {
    render(ClubTags, { club: club({ has_discord: true, applications: 'discord' }) });
    expect(screen.getByText('Discord')).toBeInTheDocument();
    expect(screen.getByText('Quick apply')).toBeInTheDocument();
  });

  it('is translated', () => {
    language.japaneseEnabled = true;
    language.choose('ja');
    try {
      render(ClubTags, { club: club({ applications: 'discord' }) });
      expect(screen.getByText('かんたん応募')).toBeInTheDocument();
    } finally {
      language.choose('en');
      language.japaneseEnabled = false;
    }
  });
});
