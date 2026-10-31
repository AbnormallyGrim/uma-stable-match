// Member list tests. All data is made up.

import { describe, expect, it } from 'vitest';

import type { ClubMember, ClubTrainer } from './api';
import { rosterRows } from './roster';

const member = (pace: number | null): ClubMember => ({ pace_per_day: pace, daily_fans: [] });

const trainer = (pace: number): ClubTrainer => ({
  trainer_id: 123456789012,
  pace_per_day: pace,
  daily_fans: null,
  in_club: false,
  your_place: null,
  club_size: null,
});

describe('rosterRows', () => {
  it('numbers ranked members and leaves new ones without a place', () => {
    const rows = rosterRows({ members: [member(9), member(5), member(null)], trainer: null });
    expect(rows.map((r) => [r.place, r.pace, r.gap, r.you])).toEqual([
      [1, 9, null, false],
      [2, 5, null, false],
      [null, null, null, false],
    ]);
  });

  it('puts the trainer after everyone faster, ahead of ties', () => {
    const rows = rosterRows({
      members: [member(9), member(6), member(6), member(2), member(null)],
      trainer: trainer(6),
    });
    expect(rows.map((r) => [r.place, r.you])).toEqual([
      [1, false],
      [2, true],
      [3, false],
      [4, false],
      [5, false],
      [null, false],
    ]);
    expect(rows.map((r) => r.gap)).toEqual([3, null, 0, 0, -4, null]);
  });

  it('puts a trainer faster than everyone first, and slower than everyone last of the ranked', () => {
    const members = [member(5), member(null)];
    expect(rosterRows({ members, trainer: trainer(8) })[0]!.you).toBe(true);
    expect(rosterRows({ members, trainer: trainer(1) }).map((r) => r.you)).toEqual([
      false,
      true,
      false,
    ]);
  });
});
