# Frontend

The Uma Stable Match web interface, built with [Svelte 5](https://svelte.dev) and TypeScript on [Vite](https://vite.dev). It's a single-page app; the Rust server will serve the built files (#40).

For now it's a placeholder page that checks whether the API server is reachable. The search form and results come in later issues.

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
index.html          Page shell
src/main.ts         Mounts the app
src/App.svelte      Root component
src/lib/api.ts      Typed API client
src/*.test.ts       Component tests
src/setup-tests.ts  Adds jest-dom matchers to Vitest
vite.config.ts      Vite, the /api proxy and Vitest settings
```

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
