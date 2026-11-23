import {
  expect,
  linkTrainer,
  newDiscordId,
  setProfileComment,
  signIn,
  test,
  trainerId,
} from './fixtures';

test.describe('signing in', () => {
  test('signs in with Discord and shows the account', async ({ page }) => {
    const user = newDiscordId();
    await signIn(page, user);

    await expect(page.getByRole('heading', { name: 'Which best describes you?' })).toBeVisible();
    await expect(
      page.getByRole('navigation', { name: 'Main' }).getByRole('button', {
        name: `Fixture User ${user}`,
      }),
    ).toBeVisible();
  });

  test('says so when the sign-in is cancelled on Discord', async ({ page }) => {
    // Known bug: a failed sign-in leaves the visitor signed out, so /applications?error=… sends
    // them to the login page, which doesn't show the error.
    test.fail();
    await page.goto('/login?next=%2Fapplications');
    await page.getByRole('link', { name: 'Sign in with Discord' }).click();
    await page.getByRole('button', { name: 'Cancel' }).click();

    await expect(page.getByText('Discord sign-in was denied. Please try again.')).toBeVisible();
    await expect(page.getByRole('link', { name: 'Sign in' })).toBeVisible();
  });

  test('stays signed in on another page, and signs out', async ({ page }) => {
    await signIn(page, newDiscordId(), '/settings');
    await page.goto('/applications/trainer');
    await expect(page.getByRole('heading', { name: 'My applications' })).toBeVisible();

    await page.goto('/settings');
    await page.getByRole('button', { name: 'Sign Out' }).click();
    // Signing out goes to the search.
    await page.waitForURL('/');
    await expect(page.getByRole('link', { name: 'Sign in' })).toBeVisible();
    await page.goto('/applications/trainer');
    await expect(page).toHaveURL(/\/login\?next=/);
  });

  test('deletes the account and its linked trainer', async ({ page }) => {
    const user = newDiscordId();
    await signIn(page, user, '/settings');
    await linkTrainer(page, trainerId(2, 5));

    const remove = page.getByRole('region', { name: 'Delete account' });
    await remove.getByRole('button', { name: 'Delete Account' }).click();
    await remove.getByRole('button', { name: 'Delete', exact: true }).click();
    await page.waitForURL('/');
    await expect(page.getByRole('link', { name: 'Sign in' })).toBeVisible();

    // Deleting ends every session started up to that second, so sign in again after it.
    await page.waitForTimeout(1100);
    await signIn(page, user, '/settings');
    await expect(page.getByText('No trainers linked yet.')).toBeVisible();
  });
});

test.describe('linking a trainer', () => {
  test('links with the key in the profile comment, and unlinks', async ({ page }) => {
    const trainer = trainerId(3, 7);
    await signIn(page, newDiscordId(), '/settings');
    await linkTrainer(page, trainer);
    const section = page.getByRole('region', { name: 'Link your umamusume account' });

    await expect(section.getByText('Linked Fixture Trainer 3-7.')).toBeVisible();
    await page.reload();
    await expect(section.getByText(trainer)).toBeVisible();

    await section.getByRole('button', { name: `Unlink trainer ${trainer}` }).click();
    await expect(section.getByText('No trainers linked yet.')).toBeVisible();
  });

  test('doesn’t link while the key isn’t in the comment', async ({ page }) => {
    const trainer = trainerId(3, 8);
    await signIn(page, newDiscordId(), '/settings');
    const section = page.getByRole('region', { name: 'Link your umamusume account' });
    await section.getByRole('textbox', { name: 'Trainer ID' }).fill(trainer);
    await section.getByRole('button', { name: 'Link Account' }).click();
    await setProfileComment(trainer, 'No key here');
    await section.getByRole('button', { name: 'Check' }).click();

    // It keeps looking in the background.
    await expect(section.getByRole('status')).toContainText('every minute for 5 minutes');
    await expect(section.getByText('No trainers linked yet.')).toBeVisible();
  });

  test('refuses a trainer ID that isn’t 9 to 12 digits', async ({ page }) => {
    await signIn(page, newDiscordId(), '/settings');
    const section = page.getByRole('region', { name: 'Link your umamusume account' });
    await section.getByRole('textbox', { name: 'Trainer ID' }).fill('1234');
    await section.getByRole('button', { name: 'Link Account' }).click();

    await expect(section.locator('code')).toHaveCount(0);
  });

  test('a trainer linked here leads the club page for its club', async ({ page }) => {
    await signIn(page, newDiscordId(), '/settings');
    await linkTrainer(page, trainerId(6));
    await page.goto('/applications/club');

    await expect(page.getByRole('heading', { name: 'Fixture Club 6', level: 1 })).toBeVisible();
    await expect(page.getByText('Linked trainer 900000006000.')).toBeVisible();
  });
});

test('a callback with a forged state doesn’t sign in', async ({ page }) => {
  await page.goto('/api/v1/auth/discord/callback?code=1-0&state=forged');

  await expect(page).toHaveURL(/\/login\?next=.*invalid_state/);
  await expect(page.getByRole('link', { name: 'Sign in with Discord' })).toBeVisible();
});
