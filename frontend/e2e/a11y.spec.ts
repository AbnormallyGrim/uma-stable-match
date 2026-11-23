import { createRequire } from 'node:module';

import type { Page } from '@playwright/test';

import { expect, newDiscordId, signIn, test, trainerId } from './fixtures';

const axePath = createRequire(import.meta.url).resolve('axe-core');

// The page's Content Security Policy would block the injected axe script.
test.use({ bypassCSP: true });

/** axe's violations on the whole page, in both themes, as `rule: target` lines. */
async function violations(page: Page): Promise<string[]> {
  await page.addScriptTag({ path: axePath });
  // Colours check mid-transition after a theme switch otherwise.
  await page.addStyleTag({ content: '*, *::before, *::after { transition: none !important; }' });
  const found: string[] = [];
  for (const theme of ['light', 'dark']) {
    await page.evaluate((theme) => (document.documentElement.dataset.theme = theme), theme);
    const results = await page.evaluate(() => window.axe.run(document));
    for (const violation of results.violations) {
      for (const node of violation.nodes) {
        found.push(`${theme} ${violation.id}: ${node.target.join(' ')}`);
      }
    }
  }
  return found;
}

declare global {
  interface Window {
    axe: typeof import('axe-core');
  }
}

const pages: [string, string, string][] = [
  ['search', '/', 'Find your club'],
  ['club page', `/club/5?trainer=${trainerId(5, 3)}`, 'Fixture Club 5'],
  ['wishlist', '/wishlist', 'Help decide what gets built next'],
  ['recruitment board', '/recruitment-board', 'Trainers looking for a club'],
  ['sign in', '/login', 'Sign in to Uma Stable Match'],
];

for (const [name, path, heading] of pages) {
  test(`the ${name} page has no accessibility violations`, async ({ page }) => {
    await page.route('https://strawpoll.com/**', (route) => route.fulfill({ body: '' }));
    await page.goto(path);
    await expect(page.getByRole('heading', { name: heading, level: 1 })).toBeVisible();

    expect(await violations(page)).toEqual([]);
  });
}

test('search results have no accessibility violations', async ({ page }) => {
  await page.goto('/');
  const form = page.getByRole('form', { name: 'Find clubs' });
  await form.getByText('Daily pace', { exact: true }).click();
  await form.getByLabel('Daily fan gain (millions)').fill('3');
  await form.getByRole('button', { name: 'Find clubs' }).click();
  await expect(page.getByRole('heading', { name: 'Top picks' })).toBeVisible();

  expect(await violations(page)).toEqual([]);
});

for (const [name, path, heading] of [
  ['applications', '/applications', 'Which best describes you?'],
  ['my applications', '/applications/trainer', 'My applications'],
  ['your club', '/applications/club', 'Your club'],
  ['settings', '/settings', 'Settings'],
]) {
  test(`the ${name} page has no accessibility violations`, async ({ page }) => {
    await signIn(page, newDiscordId(), path);
    await expect(page.getByRole('heading', { name: heading, level: 1 })).toBeVisible();

    expect(await violations(page)).toEqual([]);
  });
}
