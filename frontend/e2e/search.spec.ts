import { expect, test, trainerId } from './fixtures';

test.describe('search', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/');
  });

  test('a daily pace search shows top picks and the results table', async ({ page }) => {
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

  test('a trainer ID search shows the pace and compares the current club', async ({ page }) => {
    const form = page.getByRole('form', { name: 'Find clubs' });
    await form.getByRole('textbox', { name: 'Trainer ID' }).fill(trainerId(10, 3));
    await form.getByRole('button', { name: 'Find clubs' }).click();

    const results = page.getByRole('region', { name: 'Results' });
    await expect(results.getByRole('heading', { name: 'Top picks' })).toBeVisible();
    await expect(results.getByText('2.6M/day').first()).toBeVisible();
    const yourClub = results.getByRole('region', { name: 'Your club' });
    await expect(
      yourClub.getByRole('link', { name: 'Fixture Club 10', exact: true }),
    ).toBeVisible();
    // The current club is never a pick.
    await expect(results.getByRole('article').filter({ hasText: 'Fixture Club 10' })).toHaveCount(
      0,
    );
  });

  test('a trainer ID that isn’t 9 to 12 digits is refused', async ({ page }) => {
    const form = page.getByRole('form', { name: 'Find clubs' });
    await form.getByRole('textbox', { name: 'Trainer ID' }).fill('1234');
    await form.getByRole('button', { name: 'Find clubs' }).click();

    await expect(form.getByRole('textbox', { name: 'Trainer ID' })).toHaveAttribute(
      'aria-invalid',
      'true',
    );
    await expect(page.getByRole('heading', { name: 'Top picks' })).toHaveCount(0);
  });

  test('a trainer ID uma.moe doesn’t know shows an error', async ({ page }) => {
    const form = page.getByRole('form', { name: 'Find clubs' });
    await form.getByRole('textbox', { name: 'Trainer ID' }).fill('123456789012');
    await form.getByRole('button', { name: 'Find clubs' }).click();

    await expect(page.getByRole('alert')).toBeVisible();
    await expect(page.getByRole('heading', { name: 'Top picks' })).toHaveCount(0);
  });

  test('the approval filter lists only clubs that take applications', async ({ page }) => {
    const form = page.getByRole('form', { name: 'Find clubs' });
    await form.getByText('Daily pace', { exact: true }).click();
    await form.getByLabel('Daily fan gain (millions)').fill('3');
    await form.getByRole('button', { name: 'Approval' }).click();
    await form.getByRole('button', { name: 'Find clubs' }).click();

    const picks = page.getByRole('article');
    await expect(picks.first()).toBeVisible();
    // Every fourth fixture club takes applications.
    for (const name of await picks.evaluateAll((cards) =>
      cards.map((card) => card.getAttribute('aria-label') ?? ''),
    )) {
      expect(Number(name.replace('Fixture Club ', '')) % 4).toBe(0);
    }
  });

  test('the club page link from a pick places the trainer among its members', async ({ page }) => {
    const form = page.getByRole('form', { name: 'Find clubs' });
    await form.getByRole('textbox', { name: 'Trainer ID' }).fill(trainerId(10, 3));
    await form.getByRole('button', { name: 'Find clubs' }).click();

    await page
      .getByRole('link', { name: /^See how you fit in Fixture Club/ })
      .first()
      .click();
    await expect(page).toHaveURL(/\/club\/\d+\?trainer=900000010003$/);
    await expect(page.getByRole('heading', { level: 1 })).toContainText('Fixture Club');
    await expect(
      page.getByRole('region', { name: 'Members' }).getByText('You', { exact: true }),
    ).toBeVisible();
  });
});
