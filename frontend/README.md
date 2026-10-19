# Frontend

The Uma Stable Match web interface, built with [Svelte 5](https://svelte.dev) and TypeScript on [Vite](https://vite.dev). It's a single-page app; the Rust server will serve the built files (#40).

It has the search form, a summary of each search, the top ten picks as cards, and every scored club in a sortable table.

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
src/lib/runner.ts             Search state machine: loading, retries and countdowns
src/lib/states.ts             Error messages and empty-result hints
src/lib/SearchStatus.svelte   Loading, waiting and error messages
src/lib/DataStatus.svelte     How fresh the club data is, from /api/v1/status
src/lib/ThemeToggle.svelte    System / Light / Dark switch
src/lib/theme.ts              Saving and applying the theme choice
src/lib/api.ts                Typed API client
src/lib/search.ts             Search validation, URL parameters and "remember me"
src/lib/format.ts             Number, rank trend, label and time formatting
src/lib/Results.svelte        Results view: summary, top picks and all results
src/lib/ResultsTable.svelte   Sortable, searchable table of every scored club
src/lib/ResultsSummary.svelte Pace, clubs analysed, data age and balance setting
src/lib/ClubCard.svelte       One top pick
src/lib/TierBadge.svelte      Tier icon, with a fallback for unknown tiers
src/lib/test-data.ts          Made-up data for component tests
src/app.css                   Global styles and light and dark colour tokens
src/assets/tiers/             Club tier icons (Cygames artwork, not MIT licensed)
src/lib/SearchForm.svelte     The search form
src/**/*.test.ts              Unit and component tests
src/setup-tests.ts            Adds jest-dom matchers to Vitest
vite.config.ts                Vite, the /api proxy and Vitest settings
```

## Search form

The form sits in a card. The "Search by" switch is at the top, then the field on the left and the options on the right, with **Find clubs** at the bottom right. Below 720 pixels it becomes a single column, and the inputs and button use the full width.

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

## Results view

After a search, the page shows a summary and the top ten picks.

- **Summary:** your pace, how many clubs were analysed (the API's `total`), how long ago the data was updated (with the exact time on hover), and the balance setting used.
- **Each card:**
  - **Header:** the pick number (gold, silver and bronze for the top three), the tier icon, and the club name linking to uma.moe in a new tab
  - **Details:** the monthly rank and trend, open spots, join style and playstyle tags, and the description
  - **How you'd compare:** your place, the club's median per day, and the fit (or the score, with balance on)
  - **With balance on:** the balance score as a bar, passengers and top-three share
- **Layout:** on narrow screens, the numbers move below the club details.
- **Screen readers:** after a search, only a short "Found N clubs" message is announced, not the whole list. Rank trends have spoken descriptions, such as "Up 16 places since last month".

### Tier icons

Tiers are shown with the game's own icons, D to SS, from [`src/assets/tiers/`](src/assets/tiers/README.md). They're © Cygames, Inc. and excluded from the MIT licence; that folder's README has the details. Each icon is under Vite's 4 kB inline limit, so it's bundled into the JavaScript as a data URL. A content security policy for the page must allow `img-src data:`.

### All results table

Below the cards, every scored club is listed in a table. The app asks the API for up to 100 clubs (a scan loads at most 80 by default), so nothing is cut off.

| Column | Sorts first by |
|---|---|
| `#` (pick order) | Best pick first |
| Tier | Highest tier first |
| Club | A to Z |
| Rank, Last month | Best (lowest) rank first |
| Open spots | Most first |
| Join style | A to Z |
| Median/day | Highest first |
| You'd be | Nearest the top of the club first |
| Balance *(balance on)* | Most even first |
| Passengers, Top 3 share *(balance on)* | Fewest or lowest first |
| Fit, or Score *(balance on)* | Best (lowest) first |

- **Sorting:** select a column header to sort by it. Select it again to reverse the order. Missing values always sort last, and ties keep pick order. Each header is a button with `aria-sort`, so sorting works by keyboard and screen readers announce it.
- **Search:** filters by club name as you type. "Showing X of Y clubs" updates as you go.
- **Hide closed clubs:** only appears when the search included closed clubs, and is on by default.
- **Narrow screens:** the table scrolls sideways inside its own region. The region can be focused, so it also scrolls with the keyboard. That needs a `svelte-ignore` for Svelte's `a11y_no_noninteractive_tabindex` rule, because axe requires scrollable regions to be focusable. On a desktop the table fits without scrolling, even with balance columns.

### Themes

Colours are CSS custom properties in `src/app.css`, and components only use these tokens. Each token holds both values, as `light-dark(light, dark)`, and the root's `color-scheme` decides which one applies:

| `<html>` | `color-scheme` | Result |
|---|---|---|
| No `data-theme` | `light dark` | Follows the system setting |
| `data-theme="light"` | `light` | Always light |
| `data-theme="dark"` | `dark` | Always dark |

`light-dark()` needs Chrome or Edge 123, Firefox 120 or Safari 17.5, or newer.

The **System / Light / Dark** switch in the header (`ThemeToggle.svelte`) sets the attribute. It saves Light or Dark in local storage under `uma-stable-match:theme`, and choosing System deletes it. It's a group of radio buttons labelled "Theme", so arrow keys and screen readers work, and keyboard focus shows a ring.

A small inline script in `index.html` applies a saved theme before the page renders, so it never flashes the wrong theme while the app loads. When the server adds a content security policy (#40), it must allow this script, by hash or by moving it to its own file.

### Shared styles

`src/app.css` also styles the controls used across the page, so components don't repeat them:

| Style | Use |
|---|---|
| `button` | Every button: a card-coloured background, a border and a hover state. Disabled buttons are dimmed |
| `button.primary` | The main action, such as **Find clubs**: accent background with `--on-accent` text |
| Text and search inputs | Padding, border and theme colours. `aria-invalid="true"` turns the border red |
| `accent-color` | Checkboxes, radio buttons and the slider follow the theme's accent |
| `.check` | A checkbox and its label, aligned on one line |
| `.segmented`, `.segmented.small` | A pill-shaped group of radio buttons, used for "Search by" and the theme switch. The selected option is styled with `:has(input:checked)` |
| `:focus-visible` | A 2px accent ring around whatever has keyboard focus |

A full contrast check comes with the accessibility audit.

### Search states

`src/lib/runner.ts` runs each search and tracks where it's up to. `SearchStatus.svelte` shows that state under the form.

| Situation | What the visitor sees | Retry |
|---|---|---|
| Loading | "Finding clubs…". Previous results stay, dimmed and marked `aria-busy` | – |
| First scan still running (503) | "Club data is still loading", with a countdown | Automatic after 5, 10, 20, 30, 30 and 30 seconds (about 2 minutes), or **Try now** |
| Still loading after every retry | "Club data still isn't ready" | **Try again** |
| Rate limited (429) | "Too many searches", counting down from `Retry-After` (10 seconds without it) | **Try again**, enabled when the countdown ends. Never automatic |
| Trainer not found (404) | Where to find the trainer ID in-game | None; the search needs changing |
| Not enough data (422) | Wait a few days, or search by pace | None |
| uma.moe unreachable (502), network error, timeout, other | A short explanation | **Try again** |
| Invalid search (400) | The server's message | None |
| No matches | "No clubs matched your search", with hints to include closed clubs or lower the balance strength, when those apply | – |

Starting a new search cancels any pending retry and any earlier request, so an old answer never replaces a newer one. Loading and waiting messages use `role="status"`, and errors use `role="alert"`. The dimming transition is turned off when the visitor prefers reduced motion.

The runner takes the fetch function, a change callback, the retry delays and the timer functions as options. Its tests replace them, so they can check retries and countdowns to the second with fake timers.

### Data status

Under the heading, `DataStatus.svelte` shows how fresh the club data is, using `/api/v1/status`.

| Server state | Line shown |
|---|---|
| Data ready | "Club data updated 3 hours ago." The exact time shows on hover |
| Refreshing | "Club data updated 3 hours ago; refreshing now…" |
| First scan running | "Loading club data for the first time…" |
| Last refresh failed | The line, then "The last refresh didn't finish (…)" with the server's public-safe message, in the warning colour |
| Status request failed | Nothing |

It checks when the page loads, then every 60 seconds (`POLL_MS`). While a scan runs, or before any data exists, it checks every 5 seconds (`FAST_POLL_MS`). It doesn't check while the tab is hidden, and checks straight away when the tab becomes visible again.

When it sees a newly finished scan, it calls `onscanready`. If a search is waiting for the first scan, the app retries it straight away instead of waiting out the countdown.

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
