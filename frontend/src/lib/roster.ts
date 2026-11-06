/** The club page's member list, with the trainer added where they'd rank. */

import type { ClubDetails } from './api';

/** One row: a member or the trainer, with their place, or `null` for members too new to rank. */
export interface RosterRow {
  you: boolean;
  /** The member's index in `members`, or `null` for the trainer. */
  member: number | null;
  name: string | null;
  place: number | null;
  pace: number | null;
  /** How far the pace is above or below the trainer's, or `null` without a trainer. */
  gap: number | null;
}

/**
 * Members fastest first, with the trainer after everyone faster than them, as the server ranks
 * places. Members too new to rank stay last, without a place.
 */
export function rosterRows(details: Pick<ClubDetails, 'members' | 'trainer'>): RosterRow[] {
  const yours = details.trainer?.pace_per_day ?? null;
  const rows: RosterRow[] = details.members.map((member, index) => ({
    you: false,
    member: index,
    name: member.name,
    place: null,
    pace: member.pace_per_day,
    gap: yours !== null && member.pace_per_day !== null ? member.pace_per_day - yours : null,
  }));
  if (yours !== null) {
    const faster = rows.filter((row) => row.pace !== null && row.pace > yours).length;
    rows.splice(faster, 0, {
      you: true,
      member: null,
      name: null,
      place: null,
      pace: yours,
      gap: null,
    });
  }
  let place = 0;
  for (const row of rows) {
    if (row.pace !== null) {
      row.place = ++place;
    }
  }
  return rows;
}

/** One row of the compare page's list: a member or one of the compared trainers. */
export interface CompareRow {
  /** The member's index in `members`, or `null` for a compared trainer. */
  member: number | null;
  /** The compared trainer's index, or `null` for a member. */
  candidate: number | null;
  name: string | null;
  place: number | null;
  pace: number | null;
}

/**
 * Members fastest first with every compared trainer after everyone faster than them, ahead of
 * ties, all ranked together. Members too new to rank stay last, without a place.
 */
export function compareRows(
  members: Pick<ClubDetails['members'][number], 'name' | 'pace_per_day'>[],
  paces: number[],
): CompareRow[] {
  const rows: CompareRow[] = members.map((member, index) => ({
    member: index,
    candidate: null,
    name: member.name,
    place: null,
    pace: member.pace_per_day,
  }));
  paces
    .map((pace, candidate) => ({ pace, candidate }))
    .sort((a, b) => b.pace - a.pace || a.candidate - b.candidate)
    .forEach(({ pace, candidate }) => {
      const faster = rows.filter((row) => row.pace !== null && row.pace > pace).length;
      rows.splice(faster, 0, { member: null, candidate, name: null, place: null, pace });
    });
  let place = 0;
  for (const row of rows) {
    if (row.pace !== null) {
      row.place = ++place;
    }
  }
  return rows;
}
