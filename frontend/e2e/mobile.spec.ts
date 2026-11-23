import { expect, test } from './fixtures';

test('the navbar folds its tools behind one button on a narrow screen', async ({ page }) => {
  await page.goto('/');
  const nav = page.getByRole('navigation', { name: 'Main' });
  await expect(nav.getByRole('button', { name: "What's new" })).toBeHidden();

  await nav.getByRole('button', { name: 'More options' }).click();
  await expect(page.getByRole('button', { name: "What's new" })).toBeVisible();
});

test('a daily pace search works on a narrow screen', async ({ page }) => {
  await page.goto('/');
  const form = page.getByRole('form', { name: 'Find clubs' });
  await form.getByText('Daily pace', { exact: true }).click();
  await form.getByLabel('Daily fan gain (millions)').fill('3');
  await form.getByRole('button', { name: 'Find clubs' }).click();

  await expect(page.getByRole('heading', { name: 'Top picks' })).toBeVisible();
  const width = await page.evaluate(() => document.documentElement.scrollWidth);
  expect(width).toBeLessThanOrEqual(page.viewportSize()!.width);
});
