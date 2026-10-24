// Made-up clubs and responses for component tests.

import type { Club, Recommendations, YourClub } from './api';

export function club(overrides: Partial<Club> = {}): Club {
  return {
    circle_id: 1,
    name: 'Example Club',
    url: 'https://uma.moe/circles/1',
    tier: 'A+',
    join_style: 'open',
    policy: 2,
    member_count: 25,
    open_spots: 5,
    monthly_rank: 111,
    last_month_rank: 127,
    comment: 'Made-up description',
    your_place: 9,
    club_size: 26,
    ranked_members: 24,
    median_per_day: 6_400_000,
    fit: 0.0562,
    score: 0.1062,
    balance: null,
    ...overrides,
  };
}

export function recommendations(overrides: Partial<Recommendations> = {}): Recommendations {
  return {
    trainer: { trainer_id: null, pace_per_day: 7_600_000 },
    options: { balance_weight: null, include_closed: false, include_full: false, limit: 50 },
    updated_at: '2026-03-18T09:00:00Z',
    month: '2026-03',
    coverage: { shortlisted: 0, scored: 0, pending: 0 },
    total: 0,
    clubs: [],
    your_club: null,
    ...overrides,
  };
}

export function yourClub(overrides: Partial<YourClub> = {}): YourClub {
  return {
    ...club({
      circle_id: 99,
      name: 'Home Club',
      url: 'https://uma.moe/circles/99',
      tier: 'S',
      join_style: 'closed',
      member_count: 30,
      open_spots: 0,
      your_place: 24,
      club_size: 30,
      median_per_day: 11_200_000,
      fit: 0.4321,
      score: 0.4821,
    }),
    result_rank: 37,
    ...overrides,
  };
}
