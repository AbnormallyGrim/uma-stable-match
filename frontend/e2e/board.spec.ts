import { expect, linkTrainer, newDiscordId, signIn, test, trainerId } from './fixtures';

test('a trainer posts a listing that a signed-out visitor sees, then takes it down', async ({
  page,
  otherPage: visitor,
}) => {
  await signIn(page, newDiscordId(), '/settings');
  await linkTrainer(page, trainerId(20, 6));
  await page.goto('/recruitment-board?post=1');
  const form = page.getByRole('dialog', { name: 'My listing' });
  await form.getByRole('combobox', { name: 'Lowest club rank' }).click();
  await page.getByRole('option', { name: 'B', exact: true }).click();
  await form.getByRole('combobox', { name: 'Highest club rank' }).click();
  await page.getByRole('option', { name: 'A', exact: true }).click();
  await form.getByRole('button', { name: 'Post listing' }).click();
  await expect(form.getByRole('region', { name: 'Fixture Trainer 20-6' })).toContainText('B to A');

  await visitor.goto('/recruitment-board');
  const board = visitor.getByRole('region', { name: 'Trainers looking for a club' });
  await expect(board.getByRole('status').filter({ hasText: 'listing' })).toHaveText('1 listing');
  await expect(board.getByText('Fixture Trainer 20-6')).toBeVisible();

  await form.getByRole('button', { name: 'Delete listing' }).click();
  await visitor.reload();
  await expect(board.getByRole('status').filter({ hasText: 'listings' })).toHaveText('0 listings');
});

test('posting needs a linked trainer', async ({ page }) => {
  await signIn(page, newDiscordId(), '/recruitment-board');
  await page.getByRole('button', { name: 'Post a listing' }).click();
  const form = page.getByRole('dialog', { name: 'My listing' });
  await form.getByRole('button', { name: 'Post listing' }).click();

  await expect(form.getByRole('alert')).toContainText(/link/i);
});
