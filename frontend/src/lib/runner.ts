import { ApiError, type RecommendationQuery, type Recommendations } from './api';
import { SCAN_STILL_LOADING, describeError, type ErrorMessage } from './states';

export const RETRY_DELAYS = [5, 10, 20, 30, 30, 30];
export const DEFAULT_RATE_LIMIT_WAIT = 10;

export type SearchPhase =
  | { phase: 'idle' }
  | { phase: 'loading' }
  | { phase: 'done' }
  | { phase: 'waiting-for-scan'; retryIn: number }
  | { phase: 'rate-limited'; retryIn: number }
  | { phase: 'error'; message: ErrorMessage };

export interface SearchState {
  status: SearchPhase;
  result: Recommendations | null;
}

export interface Timers {
  setTimeout: (fn: () => void, ms: number) => unknown;
  clearTimeout: (id: unknown) => void;
}

export interface RunnerOptions {
  fetch: (query: RecommendationQuery, signal: AbortSignal) => Promise<Recommendations>;
  onchange: (state: SearchState) => void;
  delays?: number[];
  timers?: Timers;
}

export class SearchRunner {
  private state: SearchState = { status: { phase: 'idle' }, result: null };
  private query: RecommendationQuery | null = null;
  private attempt = 0;
  private controller: AbortController | null = null;
  private timer: unknown = null;
  private readonly fetchFn: RunnerOptions['fetch'];
  private readonly onchange: RunnerOptions['onchange'];
  private readonly delays: number[];
  private readonly timers: Timers;

  constructor(options: RunnerOptions) {
    this.fetchFn = options.fetch;
    this.onchange = options.onchange;
    this.delays = options.delays ?? RETRY_DELAYS;
    this.timers = options.timers ?? {
      setTimeout: (fn, ms) => globalThis.setTimeout(fn, ms),
      clearTimeout: (id) => globalThis.clearTimeout(id as ReturnType<typeof setTimeout>),
    };
  }

  run(query: RecommendationQuery) {
    this.query = query;
    this.attempt = 0;
    void this.send();
  }

  retryNow() {
    const { status } = this.state;
    if (this.query === null) return;
    if (status.phase === 'rate-limited' && status.retryIn > 0) return;
    if (status.phase === 'loading') return;
    void this.send();
  }

  dispose() {
    this.stopTimer();
    this.controller?.abort();
  }

  private async send() {
    const query = this.query;
    if (query === null) return;
    this.stopTimer();
    this.controller?.abort();
    const controller = new AbortController();
    this.controller = controller;
    this.update({ phase: 'loading' });

    try {
      const result = await this.fetchFn(query, controller.signal);
      if (controller.signal.aborted) return;
      this.attempt = 0;
      this.state = { status: { phase: 'done' }, result };
      this.onchange(this.state);
    } catch (error) {
      if (controller.signal.aborted) return;
      this.handleError(error);
    }
  }

  private handleError(error: unknown) {
    if (error instanceof ApiError && error.status === 503) {
      const delay = this.delays[this.attempt];
      if (delay === undefined) {
        this.update({ phase: 'error', message: SCAN_STILL_LOADING });
        return;
      }
      this.attempt += 1;
      this.countdown(
        delay,
        (retryIn) => ({ phase: 'waiting-for-scan', retryIn }),
        () => {
          void this.send();
        },
      );
      return;
    }
    if (error instanceof ApiError && error.status === 429) {
      this.countdown(
        error.retryAfter ?? DEFAULT_RATE_LIMIT_WAIT,
        (retryIn) => ({ phase: 'rate-limited', retryIn }),
        () => undefined,
      );
      return;
    }
    this.update({ phase: 'error', message: describeError(error) });
  }

  private countdown(seconds: number, phase: (retryIn: number) => SearchPhase, done: () => void) {
    this.update(phase(seconds));
    const tick = (left: number) => {
      this.timer = this.timers.setTimeout(() => {
        this.timer = null;
        const next = left - 1;
        this.update(phase(Math.max(0, next)));
        if (next <= 0) {
          done();
        } else {
          tick(next);
        }
      }, 1000);
    };
    if (seconds > 0) {
      tick(seconds);
    } else {
      done();
    }
  }

  private stopTimer() {
    if (this.timer !== null) {
      this.timers.clearTimeout(this.timer);
      this.timer = null;
    }
  }

  private update(status: SearchPhase) {
    this.state = { ...this.state, status };
    this.onchange(this.state);
  }
}
