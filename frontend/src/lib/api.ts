/** Typed client for the Uma Stable Match API. */

/** How a club accepts new members. */
export type JoinStyle = 'open' | 'approval' | 'closed';

/** How evenly a club's members contribute. See the scoring README. */
export interface Balance {
  score: number;
  gini: number;
  passengers: number;
  counted: number;
  bottom_quarter_per_day: number;
  top3_share: number;
}

/** One recommended club. Lower `fit` and `score` are better. */
export interface Club {
  circle_id: number;
  name: string;
  url: string;
  tier: string | null;
  join_style: JoinStyle | null;
  policy: number | null;
  member_count: number | null;
  open_spots: number | null;
  monthly_rank: number | null;
  last_month_rank: number | null;
  comment: string | null;
  your_place: number;
  club_size: number;
  ranked_members: number;
  median_per_day: number;
  fit: number;
  score: number;
  balance: Balance | null;
}

/** Response from `GET /api/v1/recommendations`. */
export interface Recommendations {
  trainer: { trainer_id: number | null; pace_per_day: number };
  options: { balance_weight: number | null; include_closed: boolean; limit: number };
  updated_at: string;
  month: string | null;
  coverage: { shortlisted: number; scored: number; pending: number };
  total: number;
  clubs: Club[];
}

/** Response from `GET /api/v1/status`. */
export interface Status {
  scanning: boolean;
  last_scan: { finished_at: string; clubs: number; candidates: number; failed: number } | null;
  last_error: { at: string; message: string } | null;
}

/** Look up a trainer's pace, or give it directly in fans per day. */
export type PaceSource = { trainerId: string } | { pace: number };

/** What to ask `/api/v1/recommendations` for. Unset options use the server's defaults. */
export interface RecommendationQuery {
  source: PaceSource;
  balanceWeight?: number | null;
  includeClosed?: boolean;
  limit?: number;
}

/** Why a request failed. */
export type ApiErrorKind = 'http' | 'network' | 'timeout' | 'invalid-response';

/** A failed API request. `status` and `retryAfter` (seconds) are set for `http` errors. */
export class ApiError extends Error {
  readonly kind: ApiErrorKind;
  readonly status: number | null;
  readonly retryAfter: number | null;

  constructor(
    kind: ApiErrorKind,
    message: string,
    status: number | null = null,
    retryAfter: number | null = null,
  ) {
    super(message);
    this.name = 'ApiError';
    this.kind = kind;
    this.status = status;
    this.retryAfter = retryAfter;
  }
}

/** Per-request options. `fetch` can be replaced in tests. */
export interface RequestOptions {
  signal?: AbortSignal;
  timeoutMs?: number;
  fetch?: typeof fetch;
}

/** How long a request may take before it fails with a `timeout` error. */
export const DEFAULT_TIMEOUT_MS = 15_000;

/** Query parameters for a recommendations request. */
export function recommendationParams(query: RecommendationQuery): URLSearchParams {
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
  if (query.limit != null) {
    params.set('limit', String(query.limit));
  }
  return params;
}

/** Fetches the scan status. */
export function fetchStatus(options: RequestOptions = {}): Promise<Status> {
  return getJson<Status>('/api/v1/status', options);
}

/**
 * Fetches ranked clubs.
 *
 * Throws `ApiError`, or the caller's abort error when `signal` is aborted.
 */
export function fetchRecommendations(
  query: RecommendationQuery,
  options: RequestOptions = {},
): Promise<Recommendations> {
  return getJson<Recommendations>(
    `/api/v1/recommendations?${recommendationParams(query)}`,
    options,
  );
}

async function getJson<T>(path: string, options: RequestOptions): Promise<T> {
  options.signal?.throwIfAborted();
  const fetchFn = options.fetch ?? globalThis.fetch;
  const controller = new AbortController();
  let timedOut = false;
  const timer = setTimeout(() => {
    timedOut = true;
    controller.abort();
  }, options.timeoutMs ?? DEFAULT_TIMEOUT_MS);
  const forwardAbort = () => controller.abort();
  options.signal?.addEventListener('abort', forwardAbort, { once: true });

  try {
    let response: Response;
    try {
      response = await fetchFn(path, {
        headers: { Accept: 'application/json' },
        signal: controller.signal,
      });
    } catch (error) {
      if (timedOut) {
        throw new ApiError('timeout', 'The server took too long to respond.');
      }
      if (options.signal?.aborted) {
        throw error;
      }
      throw new ApiError('network', 'Could not reach the server.');
    }

    if (!response.ok) {
      throw await httpError(response);
    }
    try {
      return (await response.json()) as T;
    } catch {
      throw new ApiError('invalid-response', 'The server sent an unexpected response.');
    }
  } finally {
    clearTimeout(timer);
    options.signal?.removeEventListener('abort', forwardAbort);
  }
}

async function httpError(response: Response): Promise<ApiError> {
  const body: unknown = await response.json().catch(() => null);
  const message =
    body !== null && typeof body === 'object' && 'error' in body && typeof body.error === 'string'
      ? body.error
      : `Request failed with status ${response.status}.`;
  return new ApiError('http', message, response.status, retryAfter(response));
}

function retryAfter(response: Response): number | null {
  const value = response.headers.get('Retry-After');
  if (value === null || !/^\d+$/.test(value.trim())) {
    return null;
  }
  return Number(value.trim());
}
