// TrainerApplicationsPage tests.

import { render, screen } from '@testing-library/svelte';
import { describe, expect, it } from 'vitest';

import { t } from './i18n';
import TrainerApplicationsPage from './TrainerApplicationsPage.svelte';

describe('TrainerApplicationsPage', () => {
  it('explains the page and shows the empty table', () => {
    render(TrainerApplicationsPage);
    expect(
      screen.getByRole('heading', { level: 1, name: t('trainer.heading') }),
    ).toBeInTheDocument();
    expect(screen.getByText(t('trainer.lede'))).toBeInTheDocument();
    expect(screen.getByText(t('trainer.table.empty'))).toBeInTheDocument();
    expect(document.title).toBe(t('trainer.title'));
  });
});
