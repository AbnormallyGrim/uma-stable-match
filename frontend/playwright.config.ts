import { defineConfig, devices } from '@playwright/test';

const port = 8181;

// Builds the page and serves it from the server with fixture clubs and Turnstile's always-pass
// test keys. See the frontend README.
export default defineConfig({
  testDir: 'e2e',
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 1 : 0,
  reporter: process.env.CI ? [['list'], ['html', { open: 'never' }]] : 'list',
  use: {
    baseURL: `http://127.0.0.1:${port}`,
    trace: 'retain-on-failure',
  },
  projects: [{ name: 'chromium', use: { ...devices['Desktop Chrome'] } }],
  webServer: {
    command: 'npm run build && cargo run -p uma-stable-match-server --example fixture_server',
    url: `http://127.0.0.1:${port}/api/v1/health`,
    reuseExistingServer: !process.env.CI,
    // A cold build of the server takes a few minutes.
    timeout: 10 * 60 * 1000,
    env: {
      PORT: String(port),
      FRONTEND_DIR: 'dist',
      TURNSTILE_SITE_KEY: '1x00000000000000000000AA',
      TURNSTILE_SECRET_KEY: '1x0000000000000000000000000000000AA',
      TOKEN_SIGNING_KEY: 'end-to-end-test-signing-key-not-secret',
    },
  },
});
