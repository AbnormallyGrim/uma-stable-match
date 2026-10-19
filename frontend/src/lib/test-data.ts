import type { Club, Recommendations } from './api';

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
    options: { balance_weight: null, include_closed: false, limit: 50 },
    updated_at: '2026-03-18T09:00:00Z',
    total: 0,
    clubs: [],
    ...overrides,
  };
}
