import type { RecommendationQuery } from './api';

export type SearchMode = 'trainer' | 'pace';

export interface SearchState {
  mode: SearchMode;
  trainerId: string;
  paceMillions: string;
  balance: boolean;
  balanceWeight: number;
  includeClosed: boolean;
}

export interface SearchErrors {
  trainerId?: string;
  pace?: string;
}

export type Validation =
  { ok: true; query: RecommendationQuery } | { ok: false; errors: SearchErrors };

export const DEFAULT_BALANCE_WEIGHT = 0.5;

export const DEFAULT_SEARCH: SearchState = {
  mode: 'trainer',
  trainerId: '',
  paceMillions: '',
  balance: false,
  balanceWeight: DEFAULT_BALANCE_WEIGHT,
  includeClosed: false,
};

export const TRAINER_ID_ERROR = 'Enter your trainer ID: 9 to 12 digits.';
export const PACE_ERROR = 'Enter your daily fan gain in millions, such as 7.6.';

const REMEMBER_KEY = 'uma-stable-match:trainer-id';

export function normaliseTrainerId(value: string): string {
  return value.replace(/[\s-]/g, '');
}

export function isTrainerId(value: string): boolean {
  return /^\d{9,12}$/.test(normaliseTrainerId(value));
}

export function parsePaceMillions(value: string): number | null {
  const text = value.trim().replace(',', '.');
  if (!/^\d+(\.\d+)?$/.test(text)) {
    return null;
  }
  const millions = Number(text);
  return millions > 0 ? Math.round(millions * 1_000_000) : null;
}

export function validate(state: SearchState): Validation {
  const options = {
    balanceWeight: state.balance ? clampWeight(state.balanceWeight) : null,
    includeClosed: state.includeClosed,
  };
  if (state.mode === 'trainer') {
    if (!isTrainerId(state.trainerId)) {
      return { ok: false, errors: { trainerId: TRAINER_ID_ERROR } };
    }
    return {
      ok: true,
      query: { source: { trainerId: normaliseTrainerId(state.trainerId) }, ...options },
    };
  }
  const pace = parsePaceMillions(state.paceMillions);
  if (pace === null) {
    return { ok: false, errors: { pace: PACE_ERROR } };
  }
  return { ok: true, query: { source: { pace }, ...options } };
}

export function toUrlParams(query: RecommendationQuery): URLSearchParams {
  const params = new URLSearchParams();
  if ('trainerId' in query.source) {
    params.set('trainer_id', query.source.trainerId);
  } else {
    params.set('pace', String(query.source.pace));
  }
  if (query.balanceWeight != null) {
    params.set('balance_weight', String(query.balanceWeight));
  }
  if (query.includeClosed) {
    params.set('include_closed', 'true');
  }
  return params;
}

export function fromUrlParams(params: URLSearchParams): SearchState | null {
  const trainerId = params.get('trainer_id');
  const pace = params.get('pace');
  const weight = params.get('balance_weight');
  const parsedWeight = weight === null ? null : Number(weight);
  const balance = parsedWeight !== null && Number.isFinite(parsedWeight);
  const shared = {
    balance,
    balanceWeight: balance ? clampWeight(parsedWeight) : DEFAULT_BALANCE_WEIGHT,
    includeClosed: params.get('include_closed') === 'true',
  };
  if (trainerId !== null && isTrainerId(trainerId)) {
    return {
      ...DEFAULT_SEARCH,
      ...shared,
      mode: 'trainer',
      trainerId: normaliseTrainerId(trainerId),
    };
  }
  const paceNumber = pace === null ? NaN : Number(pace);
  if (Number.isFinite(paceNumber) && paceNumber > 0) {
    return {
      ...DEFAULT_SEARCH,
      ...shared,
      mode: 'pace',
      paceMillions: String(paceNumber / 1_000_000),
    };
  }
  return null;
}

export function loadRememberedTrainerId(storage: Storage | undefined): string | null {
  try {
    const value = storage?.getItem(REMEMBER_KEY) ?? null;
    return value !== null && isTrainerId(value) ? value : null;
  } catch {
    return null;
  }
}

export function saveRememberedTrainerId(storage: Storage | undefined, trainerId: string | null) {
  try {
    if (trainerId === null) {
      storage?.removeItem(REMEMBER_KEY);
    } else {
      storage?.setItem(REMEMBER_KEY, normaliseTrainerId(trainerId));
    }
  } catch {
    return;
  }
}

function clampWeight(weight: number): number {
  return Math.min(1, Math.max(0, weight));
}
