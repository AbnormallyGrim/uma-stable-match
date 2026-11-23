import { defineConfig, devices } from '@playwright/test';

import { APP_PORT, DISCORD_PORT, REDIS_PORT } from './e2e/ports';

// Builds the page and serves it from the server with fixture clubs, a mock Discord, a throwaway
// Redis and Turnstile's always-pass test keys. Every value here is made up. See the frontend
// README.
export default defineConfig({
  testDir: 'e2e',
  globalSetup: './e2e/global-setup.ts',
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 1 : 0,
  reporter: process.env.CI ? [['list'], ['html', { open: 'never' }]] : 'list',
  use: {
    baseURL: `http://127.0.0.1:${APP_PORT}`,
    trace: 'retain-on-failure',
  },
  projects: [
    {
      name: 'chromium',
      use: { ...devices['Desktop Chrome'] },
      testIgnore: /mobile\.spec\.ts/,
    },
    {
      name: 'mobile',
      use: { ...devices['Pixel 7'] },
      testMatch: /mobile\.spec\.ts/,
    },
  ],
  webServer: [
    {
      command: `redis-server --port ${REDIS_PORT} --bind 127.0.0.1 --save "" --appendonly no`,
      port: REDIS_PORT,
      reuseExistingServer: !process.env.CI,
    },
    {
      command: 'npm run build && cargo run -p uma-stable-match-server --example fixture_server',
      url: `http://127.0.0.1:${APP_PORT}/api/v1/health`,
      reuseExistingServer: !process.env.CI,
      // A cold build of the server takes a few minutes.
      timeout: 10 * 60 * 1000,
      env: {
        PORT: String(APP_PORT),
        FRONTEND_DIR: 'dist',
        TURNSTILE_SITE_KEY: '1x00000000000000000000AA',
        TURNSTILE_SECRET_KEY: '1x0000000000000000000000000000000AA',
        TOKEN_SIGNING_KEY: 'end-to-end-test-signing-key-not-secret',
        REDIS_URL: `redis://127.0.0.1:${REDIS_PORT}`,
        REDIS_KEY_SECRET: 'end-to-end-test-redis-key-secret-not-secret',
        PUBLIC_URL: `http://127.0.0.1:${APP_PORT}`,
        DISCORD_APP_ID: '100000000000000001',
        // RFC 8032's first test public key. Nothing calls the interactions endpoint.
        DISCORD_PUBLIC_KEY: 'd75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a',
        DISCORD_BOT_TOKEN: 'end-to-end-test-bot-token',
        DISCORD_CLIENT_SECRET: 'end-to-end-test-client-secret',
        FIXTURE_DISCORD_PORT: String(DISCORD_PORT),
        // Every test comes from the same address.
        RATE_LIMIT_PER_MINUTE: '10000',
        TRAINER_LOOKUPS_PER_MINUTE: '10000',
        DISCORD_REQUESTS_PER_MINUTE: '10000',
        DISCORD_SIGN_INS_PER_HOUR: '10000',
      },
    },
  ],
});
