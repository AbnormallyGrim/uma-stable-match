/** The search progress bar's curve. It depends only on elapsed time, so it needs nothing from the server. */

/** The share of the track the bar approaches but never reaches while a search runs. */
export const PROGRESS_CAP = 0.9;
/** Milliseconds for the bar to cover about 63% of the way to the cap. */
export const PROGRESS_TAU_MS = 4000;

/** How far along the track the bar is, from 0 to `PROGRESS_CAP`, after `elapsedMs`. */
export function progressAt(elapsedMs: number, tauMs = PROGRESS_TAU_MS): number {
  if (!(elapsedMs > 0)) {
    return 0;
  }
  return PROGRESS_CAP * (1 - Math.exp(-elapsedMs / tauMs));
}
