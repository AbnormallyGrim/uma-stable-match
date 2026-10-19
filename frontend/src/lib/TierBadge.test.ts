import { render, screen } from '@testing-library/svelte';
import { describe, expect, it } from 'vitest';

import TierBadge from './TierBadge.svelte';

describe('TierBadge', () => {
  it('shows the icon for each tier', () => {
    for (const tier of ['D', 'B+', 'A', 'SS']) {
      const { unmount } = render(TierBadge, { tier });
      const img = screen.getByRole('img', { name: `Club tier ${tier}` });
      expect(img.getAttribute('src')).toMatch(/tier-\d+\.webp/);
      unmount();
    }
  });

  it('uses the icon that matches the tier order', () => {
    render(TierBadge, { tier: 'A' });
    expect(screen.getByRole('img').getAttribute('src')).toMatch(/tier-7\.webp/);
  });

  it('sizes large and small badges', () => {
    const { unmount } = render(TierBadge, { tier: 'A' });
    expect(screen.getByRole('img')).toHaveAttribute('width', '48');
    unmount();
    render(TierBadge, { tier: 'A', size: 'small' });
    expect(screen.getByRole('img')).toHaveAttribute('width', '26');
  });

  it('falls back for unknown tiers', () => {
    for (const tier of [null, 'Z']) {
      const { unmount } = render(TierBadge, { tier });
      expect(screen.queryByRole('img')).not.toBeInTheDocument();
      expect(screen.getByText('Club tier unknown')).toBeInTheDocument();
      unmount();
    }
  });
});
