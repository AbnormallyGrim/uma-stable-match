/** Statistics about the visitor across their results, for the results summary. */

import type { Club } from './api';

const COMFORT_FROM = 0.25;
const COMFORT_TO = 0.6;

function placeShare(club: Club): number {
  return club.your_place / club.club_size;
}

/** Clubs where the visitor would sit 25% to 60% of the way down the members, the range fit aims for. */
export function comfortableFits(clubs: Club[]): number {
  return clubs.filter((club) => {
    const share = placeShare(club);
    return share >= COMFORT_FROM && share <= COMFORT_TO;
  }).length;
}

/** The middle of the visitor's place shares (place ÷ club size), or `null` without clubs. */
export function typicalPlace(clubs: Club[]): number | null {
  if (clubs.length === 0) {
    return null;
  }
  const shares = clubs.map(placeShare).sort((a, b) => a - b);
  const middle = Math.floor(shares.length / 2);
  return shares.length % 2 === 1 ? shares[middle]! : (shares[middle - 1]! + shares[middle]!) / 2;
}

/** A place share as `Top 46%`, never below `Top 1%`, or a dash for `null`. */
export function formatTopShare(share: number | null): string {
  return share === null ? '–' : `Top ${Math.max(1, Math.round(share * 100))}%`;
}
