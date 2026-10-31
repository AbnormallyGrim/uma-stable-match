/** The human check: a Cloudflare Turnstile challenge, traded for a short-lived token the API needs. */

import { ApiError } from './api';

const CHALLENGE_PATH = '/api/v1/challenge';
const TURNSTILE_SCRIPT = 'https://challenges.cloudflare.com/turnstile/v0/api.js?render=explicit';
const RENEW_BEFORE_MS = 30_000;
const CHALLENGE_TIMEOUT_MS = 60_000;

/** The parts of Turnstile's widget API used here. */
export interface Turnstile {
  render(
    container: HTMLElement,
    options: {
      sitekey: string;
      appearance: 'interaction-only';
      callback: (response: string) => void;
      'error-callback': () => void;
    },
  ): string;
  remove(widgetId: string): void;
}

/** Replaceable in tests. */
export interface HumanCheckOptions {
  fetch?: typeof fetch;
  loadTurnstile?: () => Promise<Turnstile>;
  now?: () => number;
}

interface Token {
  value: string;
  expiresAt: number;
}

/**
 * Gets and keeps the API token. When the server's human check is off, `token()` resolves to `null`
 * and the check isn't asked about again.
 */
export class HumanCheck {
  private off = false;
  private token_: Token | null = null;
  private pending: Promise<string | null> | null = null;
  private readonly fetchFn: typeof fetch;
  private readonly loadTurnstile: () => Promise<Turnstile>;
  private readonly now: () => number;

  constructor(options: HumanCheckOptions = {}) {
    this.fetchFn = options.fetch ?? ((...args) => globalThis.fetch(...args));
    this.loadTurnstile = options.loadTurnstile ?? loadTurnstileScript;
    this.now = options.now ?? Date.now;
  }

  /** The token from an earlier check if it's valid for a while yet, without running a new one. */
  cached(): string | null {
    if (this.token_ && this.token_.expiresAt - this.now() > RENEW_BEFORE_MS) {
      return this.token_.value;
    }
    return null;
  }

  /** A token that's valid for a while yet, or `null` if the server needs none. */
  token(): Promise<string | null> {
    if (this.off) return Promise.resolve(null);
    const cached = this.cached();
    if (cached !== null) return Promise.resolve(cached);
    this.pending ??= this.renew().finally(() => (this.pending = null));
    return this.pending;
  }

  /** Forgets the token, after the server turned it away. */
  invalidate(): void {
    this.token_ = null;
  }

  private async renew(): Promise<string | null> {
    const siteKey = await this.siteKey();
    if (siteKey === null) {
      this.off = true;
      return null;
    }
    const response = await challenge(siteKey, await this.loadTurnstile());
    const reply = await this.fetchFn(CHALLENGE_PATH, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json', Accept: 'application/json' },
      body: JSON.stringify({ response }),
    }).catch(() => {
      throw new ApiError('network', 'Could not reach the server.');
    });
    if (reply.status === 404) {
      this.off = true;
      return null;
    }
    const body: unknown = reply.ok ? await reply.json().catch(() => null) : null;
    if (!isToken(body)) {
      throw new ApiError('human-check', 'The check failed. Try again.', reply.status);
    }
    this.token_ = { value: body.token, expiresAt: this.now() + body.expires_in * 1000 };
    return body.token;
  }

  // The site key, or `null` if the server has no human check.
  private async siteKey(): Promise<string | null> {
    const reply = await this.fetchFn(CHALLENGE_PATH, {
      headers: { Accept: 'application/json' },
    }).catch(() => {
      throw new ApiError('network', 'Could not reach the server.');
    });
    const body: unknown = reply.ok ? await reply.json().catch(() => null) : null;
    return typeof body === 'object' &&
      body !== null &&
      'site_key' in body &&
      typeof body.site_key === 'string'
      ? body.site_key
      : null;
  }
}

function isToken(body: unknown): body is { token: string; expires_in: number } {
  return (
    typeof body === 'object' &&
    body !== null &&
    'token' in body &&
    typeof body.token === 'string' &&
    'expires_in' in body &&
    typeof body.expires_in === 'number'
  );
}

// Shows the widget, which only appears if the visitor has to interact, and waits for its response.
function challenge(siteKey: string, turnstile: Turnstile): Promise<string> {
  return new Promise((resolve, reject) => {
    const container = document.createElement('div');
    container.className = 'human-check';
    document.body.append(container);
    let widget: string | null = null;
    let settled = false;
    const done = (settle: () => void) => {
      if (settled) return;
      settled = true;
      clearTimeout(timer);
      if (widget !== null) turnstile.remove(widget);
      container.remove();
      settle();
    };
    const failed = () =>
      done(() => reject(new ApiError('human-check', 'The check failed. Try again.')));
    const timer = setTimeout(failed, CHALLENGE_TIMEOUT_MS);
    try {
      widget = turnstile.render(container, {
        sitekey: siteKey,
        appearance: 'interaction-only',
        callback: (response) => done(() => resolve(response)),
        'error-callback': failed,
      });
      // Turnstile can answer before `render` returns.
      if (settled) turnstile.remove(widget);
    } catch {
      failed();
    }
  });
}

let script: Promise<Turnstile> | null = null;

function loadTurnstileScript(): Promise<Turnstile> {
  script ??= new Promise<Turnstile>((resolve, reject) => {
    const element = document.createElement('script');
    element.src = TURNSTILE_SCRIPT;
    element.async = true;
    element.onload = () => {
      const turnstile = (window as { turnstile?: Turnstile }).turnstile;
      if (turnstile) resolve(turnstile);
      else reject(new ApiError('human-check', 'The check failed to load. Try again.'));
    };
    element.onerror = () => {
      script = null;
      reject(new ApiError('human-check', 'The check failed to load. Try again.'));
    };
    document.head.append(element);
  });
  return script;
}
