import {
  approveOnDiscord,
  expect,
  linkTrainer,
  newDiscordId,
  setUpClub,
  signIn,
  test,
  trainerId,
} from './fixtures';

test.describe('club leader', () => {
  test('adds the bot after linking the club’s leader', async ({ page }) => {
    await setUpClub(page, 7);

    await expect(page).toHaveURL(/\/applications\/club\/7\?linked=1$/);
    const status = page.getByRole('region', { name: 'Bot status' });
    await expect(status.getByText('The bot is in your server')).toBeVisible();
    await expect(status.getByText('Taking applications')).toBeVisible();
    await expect(
      page.getByRole('region', { name: 'Applications' }).getByText('No applications yet'),
    ).toBeVisible();
  });

  test('can’t add the bot without leading the club', async ({ page }) => {
    await signIn(page, newDiscordId(), '/settings');
    // Member 1 of club 9, not its leader.
    await linkTrainer(page, trainerId(9, 1));
    await page.goto('/applications/club');

    await expect(page.getByRole('link', { name: 'Add to Discord' })).toHaveCount(0);
  });

  test('pauses applications', async ({ page }) => {
    await setUpClub(page, 13);
    const settings = page.getByRole('form', { name: 'Settings' });
    await settings.getByRole('switch', { name: /^Take applications/ }).uncheck({ force: true });
    await settings.getByRole('button', { name: 'Save' }).click();
    await expect(settings.getByText('Saved.')).toBeVisible();

    await page.reload();
    await expect(settings.getByRole('switch', { name: /^Take applications/ })).not.toBeChecked();
  });

  test('removes the bot from the club', async ({ page }) => {
    await setUpClub(page, 14);
    const remove = page.getByRole('region', { name: 'Remove the bot' });
    await remove.getByRole('button', { name: 'Remove the bot from this club' }).click();
    await remove.getByRole('button', { name: 'Remove', exact: true }).click();

    await expect(page.getByRole('link', { name: 'Add to Discord' })).toBeVisible();
  });
});

test.describe('applying', () => {
  test('a trainer applies with Quick Apply and both sides see the application', async ({
    page: leaderPage,
    otherPage: trainerPage,
  }) => {
    await setUpClub(leaderPage, 11);

    const trainer = newDiscordId();
    // Applications stay with the club, so each attempt applies as a different member.
    const { retry, repeatEachIndex } = test.info();
    const member = 4 + retry + repeatEachIndex * 2;
    const name = `Fixture Trainer 16-${member}`;
    const trainerIdText = trainerId(16, member);
    await signIn(trainerPage, trainer, '/settings');
    await linkTrainer(trainerPage, trainerIdText);
    await trainerPage.goto('/');
    const form = trainerPage.getByRole('form', { name: 'Find clubs' });
    await form.getByRole('textbox', { name: 'Trainer ID' }).fill(trainerIdText);
    await form.getByRole('switch', { name: 'Has Quick Apply' }).check({ force: true });
    await form.getByRole('button', { name: 'Find clubs' }).click();
    const pick = trainerPage.getByRole('article', { name: 'Fixture Club 11' });
    await pick.getByRole('link', { name: 'Quick Apply' }).click();
    await approveOnDiscord(trainerPage, trainer);

    await expect(trainerPage).toHaveURL(/\/applications\/trainer$/);
    const mine = trainerPage.getByRole('region', { name: 'Active applications' });
    await expect(mine.getByRole('row', { name: /Fixture Club 11 .* Pending/ })).toBeVisible();

    await leaderPage.reload();
    const applications = leaderPage.getByRole('region', { name: 'Applications' });
    const row = applications.getByRole('row', { name: new RegExp(`^${name} `) });
    await expect(row).toContainText('Pending');
    await row.getByRole('button', { name: `View ${name}’s application` }).click();
    const details = leaderPage.getByRole('dialog', { name: `${name}’s application` });
    await expect(details).toContainText(trainerIdText);
    await expect(details).toContainText('Fixture Club 16');

    await details.getByRole('link', { name: 'Compare in club' }).click();
    await expect(leaderPage).toHaveURL(`/applications/club/11/compare?trainers=${trainerIdText}`);
    await expect(leaderPage.getByText(name).first()).toBeVisible();
  });
});
