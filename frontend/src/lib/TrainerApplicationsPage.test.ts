// TrainerApplicationsPage tests.

import { render, screen } from '@testing-library/svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';

import { t } from './i18n';
import TrainerApplicationsPage from './TrainerApplicationsPage.svelte';

const json = (body: unknown, status = 200) =>
  new Response(JSON.stringify(body), { status, headers: { 'Content-Type': 'application/json' } });

afterEach(() => {
  vi.unstubAllGlobals();
});

describe('TrainerApplicationsPage', () => {
  it('explains the page and shows the empty table', () => {
    vi.stubGlobal('fetch', (uri: string) =>
      Promise.resolve(uri.includes('/ads/me') ? json(null) : json([])),
    );
    render(TrainerApplicationsPage);
    expect(
      screen.getByRole('heading', { level: 1, name: t('trainer.heading') }),
    ).toBeInTheDocument();
    expect(screen.getByText(t('trainer.lede'))).toBeInTheDocument();
    expect(screen.getByText(t('trainer.table.empty'))).toBeInTheDocument();
    expect(document.title).toBe(t('trainer.title'));
  });

  it('lists the applications the trainer sent', async () => {
    const sent = new Date().toISOString();
    vi.stubGlobal('fetch', (uri: string) =>
      Promise.resolve(
        uri.includes('/ads/me')
          ? json(null)
          : json([
              {
                id: 'a',
                clubId: '7',
                clubName: 'Example Club',
                trainerId: '123456789',
                status: 'pending',
                submittedAt: sent,
                ticketUrl: 'https://discord.com/channels/1/2',
              },
            ]),
      ),
    );
    render(TrainerApplicationsPage);
    expect(await screen.findByText('Example Club')).toBeInTheDocument();
    expect(screen.queryByText(t('trainer.table.empty'))).not.toBeInTheDocument();
  });
});
