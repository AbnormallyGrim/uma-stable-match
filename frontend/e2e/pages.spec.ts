import { expect, test, trainerId } from './fixtures';

test.describe('club page', () => {
  test('shows the club, its chart and its members', async ({ page }) => {
    await page.goto('/club/5');

    await expect(page.getByRole('heading', { name: 'Fixture Club 5', level: 1 })).toBeVisible();
    await expect(page.getByRole('region', { name: 'Pace this month' })).toBeVisible();
    const members = page.getByRole('region', { name: 'Members' }).getByRole('listitem');
    await expect(members).toHaveCount(20);
    await expect(members.first()).toContainText('Fixture Trainer 5-19');
  });

  test('places a trainer from the address among the members', async ({ page }) => {
    await page.goto(`/club/5?trainer=${trainerId(5, 3)}`);

    await expect(page.getByText("You'd be")).toBeVisible();
    await expect(page.getByText('17 of 20')).toBeVisible();
    await expect(page.getByText('This is your club.')).toBeVisible();
  });

  test('says so for a club that doesn’t exist', async ({ page }) => {
    await page.goto('/club/999');

    await expect(page.getByRole('alert')).toBeVisible();
    await expect(page.getByRole('region', { name: 'Members' })).toHaveCount(0);
  });

  test('goes back to the search', async ({ page }) => {
    await page.goto('/club/5');
    await page.getByRole('link', { name: '← Back to search' }).click();

    await expect(page.getByRole('heading', { name: 'Find your club' })).toBeVisible();
  });
});

test.describe('other pages', () => {
  test('the wishlist embeds its polls', async ({ page }) => {
    // The polls come from StrawPoll; the test only checks the page frames them.
    await page.route('https://strawpoll.com/**', (route) => route.fulfill({ body: '' }));
    await page.goto('/wishlist');

    await expect(
      page.getByRole('heading', { name: 'Help decide what gets built next' }),
    ).toBeVisible();
    await expect(page.getByRole('region', { name: 'Polls' }).locator('iframe')).toHaveAttribute(
      'src',
      /^https:\/\/strawpoll\.com\/embed\//,
    );
  });

  test('the recruitment board is public and starts empty', async ({ page }) => {
    await page.goto('/recruitment-board');

    await expect(
      page.getByRole('heading', { name: 'Trainers looking for a club', level: 1 }),
    ).toBeVisible();
    await expect(page.getByRole('status').filter({ hasText: 'listings' })).toHaveText('0 listings');
    await expect(page.getByRole('link', { name: 'Sign in to post a listing' })).toBeVisible();
  });

  test('the old board address goes to the board', async ({ page }) => {
    await page.goto('/applications/ads');

    await expect(page).toHaveURL(/\/recruitment-board$/);
  });

  for (const path of [
    '/applications',
    '/applications/trainer',
    '/applications/club',
    '/settings',
  ]) {
    test(`${path} asks a signed-out visitor to sign in`, async ({ page }) => {
      await page.goto(path);

      await expect(page).toHaveURL(`/login?${new URLSearchParams({ next: path })}`);
      await expect(page.getByRole('link', { name: 'Sign in with Discord' })).toBeVisible();
    });
  }
});

test.describe('navbar', () => {
  test('links to each page', async ({ page }) => {
    await page.goto('/');
    const pages = page.getByRole('navigation', { name: 'Pages' });

    await pages.getByRole('link', { name: 'Wishlist' }).click();
    await expect(page).toHaveURL(/\/wishlist$/);
    await pages.getByRole('link', { name: /Recruitment Board/ }).click();
    await expect(page).toHaveURL(/\/recruitment-board$/);
    await pages.getByRole('link', { name: 'Club finder' }).click();
    await expect(page.getByRole('heading', { name: 'Find your club' })).toBeVisible();
  });

  test('opens the release notes', async ({ page }) => {
    await page.goto('/');
    await page.getByRole('button', { name: "What's new" }).click();

    await expect(page.getByRole('dialog')).toBeVisible();
  });

  test('switches to the dark theme and remembers it', async ({ page }) => {
    await page.goto('/');
    await page.getByRole('radio', { name: 'Dark' }).check({ force: true });
    const theme = () => page.evaluate(() => document.documentElement.dataset.theme);

    expect(await theme()).toBe('dark');
    await page.reload();
    expect(await theme()).toBe('dark');
  });
});
