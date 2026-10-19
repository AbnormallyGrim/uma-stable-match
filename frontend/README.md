# Frontend

The Uma Stable Match web interface, built with [Svelte 5](https://svelte.dev) and TypeScript on [Vite](https://vite.dev). It's a single-page app; the Rust server will serve the built files (#40).

For now it has the search form and shows how many clubs were found. The results view comes in a later issue.

## Requirements

Node.js 22.22.2 or newer on the 22 line, 24.15 or newer on the 24 line, or 26 and later. Some of the tooling (jsdom and ESLint) needs these versions.

## Run

Start the API server first (see the [server README](../backend/server/README.md)), then:

```bash
cd frontend
npm install
npm run dev
```

Vite serves the page at <http://localhost:5173> and forwards every `/api` request to the API server. By default it forwards to `http://127.0.0.1:8080`, the server's default address. To use a different address, set `API_TARGET`:

```bash
API_TARGET=http://127.0.0.1:3000 npm run dev
```

## Scripts

| Command | What it does |
|---|---|
| `npm run dev` | Development server with hot reload and the `/api` proxy |
| `npm run build` | Production build into `dist/` |
| `npm run preview` | Serves the `dist/` build locally |
| `npm run check` | Type checks TypeScript and Svelte files with `svelte-check`; warnings fail the check |
| `npm run lint` | Runs ESLint, then checks formatting with Prettier |
| `npm run format` | Formats the code with Prettier |
| `npm test` | Runs the component tests once with Vitest |

CI runs `check`, `lint`, `test` and `build` on Node 22 for every push and pull request.

## Layout

```
index.html                    Page shell
src/main.ts                   Mounts the app
src/App.svelte                Root component: runs searches and keeps them in the URL
src/lib/api.ts                Typed API client
src/lib/search.ts             Search validation, URL parameters and "remember me"
src/lib/format.ts             Number, rank trend, label and time formatting
src/lib/SearchForm.svelte     The search form
src/**/*.test.ts              Unit and component tests
src/setup-tests.ts            Adds jest-dom matchers to Vitest
vite.config.ts                Vite, the /api proxy and Vitest settings
```

## Search form

The form searches by **trainer ID** or by **daily pace**, with an optional **balance scoring** toggle and strength slider (0 to 1, in steps of 0.05) and an **include closed clubs** checkbox.

| Field | Accepted | Sent to the API as |
|---|---|---|
| Trainer ID | 9 to 12 digits; spaces and dashes are ignored | `trainer_id` |
| Daily pace | A positive number of millions, such as `7.6` (a comma works as the decimal point) | `pace`, in fans per day |
| Balance scoring | Off, or a strength from 0 to 1 | `balance_weight`, only when on |
| Include closed clubs | On or off | `include_closed=true`, only when on |

Only the field for the chosen mode is checked. An invalid field gets a message linked to it with `aria-describedby`, and nothing is sent.

### Search in the URL

Each search replaces the page URL's query with the same parameters the API uses, such as `?trainer_id=123456789012&balance_weight=0.5`. When the page loads with a valid search in the URL, it fills in the form and runs the search, so searches can be bookmarked and shared. Invalid or missing parameters are ignored. If a search is still running when a new one starts, the older one is cancelled.

### Remembering the trainer ID

"Remember my trainer ID on this device" is off by default. When it's ticked at search time, the trainer ID is saved in the browser's local storage under `uma-stable-match:trainer-id`. Unticking it and searching again deletes it. Nothing else is stored. Without a search in the URL, a remembered ID fills the form, but doesn't start a search on its own. Browsers that block storage simply don't remember.

## Display formatting

`src/lib/format.ts` turns API values into display text. The functions are pure; `formatUpdated` takes the current time as a parameter, so tests can fix it.

| Function | Example |
|---|---|
| `formatMillions` / `formatPerDay` | `6400000` → `6.4M` / `6.4M/day`; `850000` → `0.85M` |
| `formatPlace` | `8, 30` → `8 of 30` |
| `formatPercent` | `0.344` → `34%` |
| `formatScore` | `0.10449` → `0.104` |
| `rankTrend` | `111, 127` → `▲16` ("Up 16 places since last month"); no last-month rank → `new` |
| `joinStyleLabel` | `approval` → `Approval` |
| `playstyleLabel` | `2` → `Laid-back`, using uma.moe's names for policy codes 1 to 17 |
| `formatUpdated` | 3 hours earlier → `3 hours ago`; a day earlier → `yesterday` |

## API client

`src/lib/api.ts` wraps the server's `/api/v1` endpoints. Its types match the responses described in the [server README](../backend/server/README.md#endpoints).

```ts
import { ApiError, fetchRecommendations } from './lib/api';

try {
  const result = await fetchRecommendations({
    source: { trainerId: '123456789012' }, // or { pace: 7_600_000 }
    balanceWeight: 0.5,
    includeClosed: false,
    limit: 50,
  });
  console.log(result.total, result.clubs[0]?.name);
} catch (error) {
  if (error instanceof ApiError && error.status === 429) {
    console.log(`Try again in ${error.retryAfter} seconds`);
  }
}
```

- **Request:** `trainerId` is a string, so long IDs keep every digit. Options left unset aren't sent, so the server's defaults apply.
- **Timeout:** requests fail with a `timeout` error after 15 seconds (`DEFAULT_TIMEOUT_MS`). Set `timeoutMs` to change it.
- **Cancelling:** pass `signal` to cancel a request. A cancelled request rejects with the browser's own `AbortError`, not an `ApiError`, so it can be ignored easily.
- **Errors:** every other failure throws an `ApiError` with a `kind`:

| `kind` | When | `status` | `message` |
|---|---|---|---|
| `http` | The server answered with an error status | The HTTP status | The server's `error` text, or a generic message |
| `network` | The server couldn't be reached | `null` | `Could not reach the server.` |
| `timeout` | No answer within the timeout | `null` | `The server took too long to respond.` |
| `invalid-response` | A success response that isn't JSON | `null` | `The server sent an unexpected response.` |

`retryAfter` holds the response's `Retry-After` header in seconds, or `null` without one. The server sends it with `429` responses.

## Tooling

- **TypeScript** runs in strict mode, with `noUncheckedIndexedAccess`.
- **ESLint** uses flat config, with the recommended JavaScript, TypeScript and Svelte rules.
- **Prettier** uses single quotes and 100-character lines. Markdown files are left alone.
- **Tests** use Vitest with jsdom and [Testing Library](https://testing-library.com/docs/svelte-testing-library/intro). Stub `fetch` with `vi.stubGlobal` rather than calling a real server.
