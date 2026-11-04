/** Feature flags, switched in code. */

/**
 * Whether the club applications pages (`/applications`) and their link in the navbar are shown.
 * While it is off, those addresses open the club finder like any other unknown address.
 */
const applicationsEnabled = import.meta.env.VITE_APPLICATIONS_ENABLED;
export const APPLICATIONS_ENABLED = applicationsEnabled == null ? false : applicationsEnabled === 'true';
