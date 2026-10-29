/** Language choice: saving it and working out which one to show. */

/** The languages the site is offered in. */
export type Language = 'en' | 'ja';

/** Local storage key for the chosen language. The website uses the same key. */
export const LANGUAGE_KEY = 'uma-stable-match:language';

/** Cookie that shares the choice between the website and the app. The website uses the same one. */
export const LANGUAGE_COOKIE = 'uma-stable-match-language';

/** Where cookies are read and written: `document`, or a stand-in in tests. */
export type CookieDocument = Pick<Document, 'cookie'> & {
  location: Pick<Location, 'hostname' | 'protocol'>;
};

/** Whether a value is a supported language. */
export function isLanguage(value: unknown): value is Language {
  return value === 'en' || value === 'ja';
}

/** The domain to share the cookie on, or `undefined` to keep it on this host only. */
export function cookieDomain(hostname: string): string | undefined {
  return hostname === 'umastable.com' || hostname.endsWith('.umastable.com')
    ? 'umastable.com'
    : undefined;
}

function readCookie(cookies: CookieDocument | undefined): string | undefined {
  try {
    const prefix = `${LANGUAGE_COOKIE}=`;
    return cookies?.cookie
      .split('; ')
      .find((pair) => pair.startsWith(prefix))
      ?.slice(prefix.length);
  } catch {
    return undefined;
  }
}

function writeCookie(cookies: CookieDocument | undefined, language: Language) {
  if (!cookies) {
    return;
  }
  const domain = cookieDomain(cookies.location.hostname);
  const secure = cookies.location.protocol === 'https:';
  try {
    cookies.cookie =
      `${LANGUAGE_COOKIE}=${language}; Path=/; Max-Age=31536000; SameSite=Lax` +
      (domain ? `; Domain=${domain}` : '') +
      (secure ? '; Secure' : '');
  } catch {
    return;
  }
}

/**
 * The saved choice: the shared cookie, then local storage, then English. Blocked storage counts
 * as nothing saved.
 */
export function loadLanguage(storage: Storage | undefined, cookies?: CookieDocument): Language {
  const shared = readCookie(cookies);
  if (isLanguage(shared)) {
    return shared;
  }
  try {
    const value = storage?.getItem(LANGUAGE_KEY);
    return isLanguage(value) ? value : 'en';
  } catch {
    return 'en';
  }
}

/** Saves the choice in local storage and the shared cookie. Storage errors are ignored. */
export function saveLanguage(
  storage: Storage | undefined,
  language: Language,
  cookies?: CookieDocument,
) {
  writeCookie(cookies, language);
  try {
    storage?.setItem(LANGUAGE_KEY, language);
  } catch {
    return;
  }
}

/** The language to show: the saved one if Japanese is enabled, otherwise English. */
export function resolveLanguage(japaneseEnabled: boolean, saved: Language): Language {
  return japaneseEnabled ? saved : 'en';
}
