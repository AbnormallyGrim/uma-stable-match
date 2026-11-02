/** The pages the navbar links to, and which one is showing. */

import type { MessageKey } from './i18n';
import { isWishlistRoute, WISHLIST_PATH } from './route';

/** One page in the navbar. */
export interface Page {
  id: 'home' | 'finder' | 'wishlist';
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

/** The website's home page, the club finder and the wishlist page. */
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
];

/** The page `pathname` belongs to. The club page is part of the finder. */
export function currentPage(pathname: string): Page['id'] {
  return isWishlistRoute(pathname) ? 'wishlist' : 'finder';
}
