// Shared fixtures for the end-to-end tests. See the frontend README.

import { createConnection } from 'node:net';

import { test as base, expect, type Page } from '@playwright/test';

import { ANNOUNCEMENT_KEY } from '../src/lib/announcement';
import { DISCORD_PORT, REDIS_PORT } from './ports';

// The same keys as `TOUR_KEY` and `BOARD_TOUR_KEY` in src/lib/tour.ts, which can't load outside
// the browser.
const TOUR_KEYS = ['uma-stable-match:tour-seen', 'uma-stable-match:board-tour-seen'];

/** Trainer ID of member `member` of fixture club `club`. Member 0 leads the club. */
export function trainerId(club: number, member = 0): string {
  return String(900_000_000_000 + club * 1000 + member);
}

let users = 0;

/** A Discord ID no other test in this run uses. Workers that restart get a new process ID. */
export function newDiscordId(): string {
  users += 1;
  return String(100_000_000_000_000_000n + BigInt(process.pid) * 1_000_000n + BigInt(users));
}

/** Sets trainer `trainer`'s profile comment, as if they'd edited it in the game. */
export async function setProfileComment(trainer: string, comment: string): Promise<void> {
  const response = await fetch(
    `http://127.0.0.1:${DISCORD_PORT}/fixture/profiles/${trainer}/comment`,
    { method: 'PUT', body: comment },
  );
  expect(response.ok).toBe(true);
}

/** Approves the sign-in on the mock Discord's page as `discordId`. */
export async function approveOnDiscord(page: Page, discordId: string): Promise<void> {
  await expect(page.getByRole('heading', { name: 'Mock Discord' })).toBeVisible();
  await page.getByLabel('Discord user ID').fill(discordId);
  await page.getByRole('button', { name: 'Authorize' }).click();
}

/** Signs in through the mock Discord as `discordId` and waits to land on `next`. */
export async function signIn(page: Page, discordId: string, next = '/applications'): Promise<void> {
  await page.goto(`/login?${new URLSearchParams({ next })}`);
  await page.getByRole('link', { name: 'Sign in with Discord' }).click();
  await approveOnDiscord(page, discordId);
  await page.waitForURL((url) => url.pathname === next);
}

/** Links `trainer` to the signed-in user from Settings, through the real profile key check. */
export async function linkTrainer(page: Page, trainer: string): Promise<void> {
  await page.goto('/settings');
  const section = page.getByRole('region', { name: 'Link your umamusume account' });
  await section.getByRole('textbox', { name: 'Trainer ID' }).fill(trainer);
  await section.getByRole('button', { name: 'Link Account' }).click();
  const key = await section.locator('code').filter({ hasText: /^uma-/ }).innerText();
  await setProfileComment(trainer, `Hello! ${key}`);
  await section.getByRole('button', { name: 'Check' }).click();
  await expect(section.getByRole('button', { name: `Unlink trainer ${trainer}` })).toBeVisible();
}

/** Signs in as a new user who leads fixture club `club`, and adds the bot to its server. */
export async function setUpClub(page: Page, club: number): Promise<string> {
  const leader = newDiscordId();
  await signIn(page, leader, '/settings');
  await linkTrainer(page, trainerId(club));
  await page.goto('/applications/club');
  const add = page.getByRole('link', { name: 'Add to Discord' });
  const remove = page.getByRole('region', { name: 'Remove the bot' });
  // A retry finds the club already set up, so starts again from no bot.
  await expect(add.or(remove)).toBeVisible();
  if (await remove.isVisible()) {
    await remove.getByRole('button', { name: 'Remove the bot from this club' }).click();
    await remove.getByRole('button', { name: 'Remove', exact: true }).click();
  }
  await add.click();
  await approveOnDiscord(page, leader);
  await expect(page.getByText('The bot was added to your server.')).toBeVisible();
  return leader;
}

/** A Redis command in the protocol's wire format. */
function command(...args: string[]): string {
  const parts = args.map((arg) => `$${Buffer.byteLength(arg)}\r\n${arg}\r\n`);
  return `*${args.length}\r\n${parts.join('')}`;
}

/** Lua that removes every key matching `ARGV[1]`. */
const CLEAR_SCRIPT =
  "for _, key in ipairs(redis.call('KEYS', ARGV[1])) do redis.call('UNLINK', key) end";

/**
 * Clears the Discord keys in the test Redis, so each run starts with no sessions, links, clubs set
 * up or listings. The server's own caches stay, so loaded clubs stay loaded.
 */
export function clearDiscord(): Promise<void> {
  return new Promise((resolve, reject) => {
    const socket = createConnection(REDIS_PORT, '127.0.0.1', () => {
      socket.write(command('EVAL', CLEAR_SCRIPT, '0', 'discord:*'));
    });
    socket.once('data', (reply) => {
      socket.end();
      if (reply.toString().startsWith('$-1')) {
        resolve();
      } else {
        reject(new Error(`Redis couldn't clear the Discord keys: ${reply}`));
      }
    });
    socket.once('error', reject);
  });
}

/** Marks the tour and the pages announcement as seen, so they don't cover the page. */
async function skipIntros(page: Page): Promise<void> {
  await page.addInitScript(
    (keys) => {
      for (const key of keys) {
        localStorage.setItem(key, '1');
      }
    },
    [...TOUR_KEYS, ANNOUNCEMENT_KEY],
  );
}

/**
 * Playwright's `test`, with the tour and the pages announcement already dismissed, and
 * `otherPage`: a page in a browser of its own, for a second person.
 */
export const test = base.extend<{ otherPage: Page }>({
  page: async ({ page }, use) => {
    await skipIntros(page);
    await use(page);
  },
  otherPage: async ({ browser }, use) => {
    const context = await browser.newContext();
    const page = await context.newPage();
    await skipIntros(page);
    await use(page);
    await context.close();
  },
});

export { expect };
