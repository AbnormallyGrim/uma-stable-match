/** Feature flags from the server's `GET /api/v1/features`, shared by every component. */

/** Which features are on. All are off until the server says otherwise. */
export const flags = $state({
  /** Whether the club applications pages and their navbar link are shown. */
  applications: false,
});
