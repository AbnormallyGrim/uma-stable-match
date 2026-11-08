// TrainerAdsTable tests. All data is made up.

import { render, screen } from '@testing-library/svelte';
import { describe, expect, it } from 'vitest';

import type { MyAd } from './ads';
import { t } from './i18n';
import TrainerAdsTable from './TrainerAdsTable.svelte';

const ad: MyAd = {
  trainerName: 'Example Trainer',
  trainerId: '123456789',
  currentClub: null,
  pace: 1_000_000,
  fansThisMonth: 10_000_000,
  rankMin: 3,
  rankMax: 8,
  discord: 'required',
  language: 'ko',
  postedAt: '2026-11-01T00:00:00Z',
  visibleUntil: '2026-11-08T00:00:00Z',
  expiresAt: '2026-12-01T00:00:00Z',
  onBoard: true,
  invites: [
    { clubId: '7', clubName: 'Example Club', sentAt: '2026-11-02T00:00:00Z' },
    { clubId: '8', clubName: null, sentAt: '2026-11-03T00:00:00Z' },
  ],
};

describe('TrainerAdsTable', () => {
  it('invites a trainer without an ad to post one', () => {
    render(TrainerAdsTable, { ad: null });
    expect(screen.getByText(t('trainer.ads.empty'))).toBeInTheDocument();
    expect(screen.getByRole('link', { name: t('ads.post') })).toHaveAttribute(
      'href',
      '/applications/ads?post=1',
    );
  });

  it('shows the ad, its status and how many invitations it has', () => {
    render(TrainerAdsTable, { ad });
    expect(screen.getByText('C to A+')).toBeInTheDocument();
    expect(screen.getByText(t('ad.language.ko'))).toBeInTheDocument();
    expect(screen.getByText(t('ads.table.discord.required'))).toBeInTheDocument();
    expect(screen.getByText(/On the board until/)).toBeInTheDocument();
    expect(screen.getByRole('cell', { name: '2' })).toBeInTheDocument();
    expect(screen.getByRole('link', { name: t('trainer.ads.manage') })).toHaveAttribute(
      'href',
      '/applications/ads?post=1',
    );
  });

  it('says when the ad has left the board', () => {
    render(TrainerAdsTable, { ad: { ...ad, onBoard: false } });
    expect(screen.getByText(t('trainer.ads.off'))).toBeInTheDocument();
  });
});
