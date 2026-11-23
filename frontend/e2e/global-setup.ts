import { clearDiscord } from './fixtures';

/** Starts each run with no Discord data, also when a running server is reused. */
export default async function globalSetup(): Promise<void> {
  await clearDiscord();
}
