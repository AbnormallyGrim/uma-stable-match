/** Whether the navbar's folded tools are open on narrow screens, shared with the product tour. */

/** Screens this narrow fold the release notes, help, language and theme behind one button. */
export const NARROW_NAV = '(width < 768px)';

/** `open` while the folded tools show and the pages pill is shrunk to the current page. */
export const navTools = $state({ open: false });

/** Whether the screen is narrow enough to fold the tools. */
export function isNarrowNav(): boolean {
  return typeof window !== 'undefined' && window.matchMedia?.(NARROW_NAV).matches === true;
}
