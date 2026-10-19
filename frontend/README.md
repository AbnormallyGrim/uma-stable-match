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
src/*.test.ts       Component tests
src/setup-tests.ts  Adds jest-dom matchers to Vitest
vite.config.ts      Vite, the /api proxy and Vitest settings
```

## Tooling

- **TypeScript** runs in strict mode, with `noUncheckedIndexedAccess`.
- **ESLint** uses flat config, with the recommended JavaScript, TypeScript and Svelte rules.
- **Prettier** uses single quotes and 100-character lines. Markdown files are left alone.
- **Tests** use Vitest with jsdom and [Testing Library](https://testing-library.com/docs/svelte-testing-library/intro). Stub `fetch` with `vi.stubGlobal` rather than calling a real server.
