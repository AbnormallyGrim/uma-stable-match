import { expect, test } from '@playwright/test';

test('a daily pace search shows top picks and the results table', async ({ page }) => {
  await page.goto('/');
  await page.getByRole('button', { name: 'Skip tour' }).click();

  const form = page.getByRole('form', { name: 'Find clubs' });
  await form.getByText('Daily pace', { exact: true }).click();
  await form.getByLabel('Daily fan gain (millions)').fill('3');
  await form.getByRole('button', { name: 'Find clubs' }).click();

  await expect(page.getByRole('heading', { name: 'Top picks' })).toBeVisible();
  await expect(page.getByRole('article').first()).toContainText('Fixture Club');

  const rows = page
    .getByRole('region', { name: 'Top results' })
    .getByRole('row')
    .filter({ hasText: 'Fixture Club' });
  await expect(rows.first()).toBeVisible();
  expect(await rows.count()).toBeGreaterThan(1);
});
