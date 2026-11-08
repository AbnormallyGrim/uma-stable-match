/** The pages the navbar links to, and which one is showing. */

import type { MessageKey } from './i18n';
import {
  ADS_PATH,
  APPLICATIONS_PATH,
  applicationsRoute,
  isLoginRoute,
  isSettingsRoute,
  isWishlistRoute,
  WISHLIST_PATH,
} from './route';

/** One page in the navbar. */
export interface Page {
  id: 'home' | 'finder' | 'wishlist' | 'applications' | 'ads';
  href: string;
  label: MessageKey;
  /** The icon's drawing: SVG elements for a 16 by 16 grid, drawn as outlines. */
  icon: string;
}

/** Where a page's address points: its protocol, host name and port. */
export type Origin = Pick<Location, 'protocol' | 'hostname' | 'port'>;

/**
 * The website's address. Run locally, on `localhost` or a name under it, it's `site.localhost` of
 * the same server.
 */
export function websiteUrl({ protocol, hostname, port }: Origin): string {
  return hostname === 'localhost' || hostname.endsWith('.localhost') || hostname === '127.0.0.1'
    ? `${protocol}//site.localhost${port ? `:${port}` : ''}`
    : 'https://umastable.com';
}

/**
 * The pages the navbar links to: the website's home page, the club finder, wishlist, applications
 * and the board of recruitment ads.
 */
export const PAGES: Page[] = [
  {
    id: 'home',
    href: websiteUrl(window.location),
    label: 'nav.home',
    icon: '<path d="M2 7.5 8 2.5l6 5M3.5 6.5v6.5h3.25v-3.5h2.5V13H12.5V6.5" />',
  },
  {
    id: 'finder',
    href: '/',
    label: 'nav.finder',
    icon: '<circle cx="7" cy="7" r="4.5" /><path d="m10.5 10.5 3.5 3.5" />',
  },
  {
    id: 'wishlist',
    href: WISHLIST_PATH,
    label: 'wishlist.link',
    icon: '<rect x="2" y="2" width="12" height="12" rx="2" /><path d="m5.25 8.25 2 2 3.5-4.5" />',
  },
  {
    id: 'applications',
    href: APPLICATIONS_PATH,
    label: 'applications.link',
    icon: '<circle cx="8" cy="5.25" r="2.5" /><path d="M3 13.5c.5-2.75 2.5-4 5-4s4.5 1.25 5 4" />',
  },
  {
    id: 'ads',
    href: ADS_PATH,
    label: 'nav.ads',
    icon: '<path d="M2.5 6.25v3.5h2.25L10 13V3L4.75 6.25H2.5Z" /><path d="M12.5 5.75a3 3 0 0 1 0 4.5" />',
  },
];

/**
 * The page `pathname` belongs to, or `null` for the settings page, which isn't in the navbar. The
 * club page is part of the finder, sign-in of applications, and the board of ads is its own page.
 */
export function currentPage(pathname: string): Page['id'] | null {
  if (isSettingsRoute(pathname)) {
    return null;
  }
  const applications = applicationsRoute(pathname, '');
  if (applications?.page === 'ads') {
    return 'ads';
  }
  if (applications || isLoginRoute(pathname)) {
    return 'applications';
  }
  return isWishlistRoute(pathname) ? 'wishlist' : 'finder';
}
