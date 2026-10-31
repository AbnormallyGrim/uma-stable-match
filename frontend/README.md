# Frontend

The Uma Stable Match web interface, built with [Svelte 5](https://svelte.dev) and TypeScript on [Vite](https://vite.dev). It's a single-page app; the Rust server will serve the built files (#40).

It has the search form, a summary of each search, the top ten picks as cards, and the 100 best fits in a sortable table.

## Requirements

Node.js 22.22.2 or newer on the 22 line, 24.15 or newer on the 24 line, or 26 and later. Some of the tooling (jsdom and ESLint) needs these versions. The main README shows [how to install Node.js](../README.md#prerequisites) on macOS, Windows and Linux.

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

In PowerShell on Windows:

```powershell
$env:API_TARGET="http://127.0.0.1:3000"; npm run dev
```

## Scripts

| Command | What it does |
|---|---|
| `npm run dev` | Development server with hot reload and the `/api` proxy |
| `npm run build` | Production build into `dist/`, which the server serves when `FRONTEND_DIR` points at it |
| `npm run preview` | Serves the `dist/` build locally |
| `npm run check` | Type checks TypeScript and Svelte files with `svelte-check`; warnings fail the check |
| `npm run lint` | Runs ESLint, then checks formatting with Prettier |
| `npm run format` | Formats the code with Prettier |
| `npm test` | Runs the component tests once with Vitest |
| `npm run e2e` | Runs the [end-to-end test](#end-to-end-test) with Playwright |

CI runs `check`, `lint`, `test` and `build` on Node 22 for every push to `main` and every pull request that changes `frontend/`.

## Layout

```
index.html                    Page shell
public/theme-init.js          Applies the saved theme before the page renders
public/favicon-32.png         Browser icon, with apple-touch-icon.png for home screens
src/main.ts                   Mounts the search, or the club page for a /club/ address
src/App.svelte                Root component: runs searches and keeps them in the URL
src/lib/route.ts              Which page an address shows, and club page addresses
src/lib/ClubPage.svelte       The club page: a club's details, its chart and member cards, with you added
src/lib/roster.ts             The club page's member cards, with you placed where you'd rank
src/lib/PaceChart.svelte      The club page chart: everyone's pace this month, with you, the median and a chosen member
src/lib/chart.ts              The chart's numbers: daily gains, fans this month, the median line and ticks
src/lib/runner.ts             Search state machine: loading, retries and countdowns
src/lib/states.ts             Error messages and empty-result hints
src/lib/HowToUse.svelte       Step-by-step instructions under the search form
src/lib/SearchStatus.svelte   Loading, waiting and error messages
src/lib/RunnerProgress.svelte The running figure that moves along a track while a search or club loads
src/lib/progress.ts           The progress curve: how far along the track the runner is
src/lib/NavBar.svelte         Top bar: the brand, linking to the website, the release notes and tour buttons, and the theme switch
src/lib/ReleaseNotes.svelte   The release notes modal: a timeline of builds, newest first
src/lib/releases.ts           The releases themselves, and remembering the newest one seen
src/lib/Tour.svelte           The product tour's overlay, highlight and instruction card
src/lib/tour.ts               The tour's steps, and remembering that it was seen
src/lib/ThemeToggle.svelte    System / Light / Dark switch, as icons with tooltips
src/lib/theme.ts              Saving and applying the theme choice
src/lib/LanguageToggle.svelte  Translate button, shown only when Japanese is enabled
src/lib/i18n.ts               `t()`, which looks up a message in the language being shown
src/lib/messages/en.ts        Every message in English, and the shape other languages follow
src/lib/messages/ja.ts        The same messages in Japanese
src/lib/language.ts           Saving the language choice, and which language to show
src/lib/language.svelte.ts    The language shown right now, shared by every component
src/lib/SiteFooter.svelte     Tips, the logo, a link to report a problem, and the disclaimer
src/lib/BrandLogo.svelte      The logo, in the version for the current theme
src/lib/api.ts                Typed API client
src/lib/features.ts           Switches on Japanese and analytics from the server's features, on every page
src/lib/human.ts              The human check: Turnstile challenge and API token
src/lib/search.ts             Search validation, URL parameters and "remember me"
src/lib/format.ts             Number, rank trend, label and time formatting
src/lib/Results.svelte        Results view: summary, top picks, notes and top results
src/lib/ResultsTable.svelte   Sortable, searchable table of the best fits
src/lib/ResultsSummary.svelte Pace, clubs analysed, comfortable fits, typical place and data age
src/lib/stats.ts              Comfortable fits and typical place, from the results
src/lib/ClubCard.svelte       One top pick
src/lib/MembersLink.svelte    "See how you fit" button to a club page, noting the search for "Back to search"
src/lib/ClubTags.svelte       A club's rank, trend, open spots and tags, on cards and the club page
src/lib/Dropdown.svelte       A themed single-choice dropdown, used for playstyle
src/lib/YourClub.svelte       The trainer's own club, compared with the top pick
src/lib/TierBadge.svelte      Tier icon, with a fallback for unknown tiers
src/lib/test-data.ts          Made-up data for component tests
src/app.css                   Global styles and light and dark colour tokens
src/assets/tiers/             Club tier icons (Cygames artwork, not MIT licensed)
src/assets/brand/             Logo and background artwork, light and dark
src/assets/runner/            The running figure's sprite strip (not MIT licensed)
src/lib/SearchForm.svelte     The search form
src/**/*.test.ts              Unit and component tests
src/a11y.test.ts              axe-core checks for the main views and states
src/contrast.test.ts          WCAG AA contrast for every colour token pair
src/setup-tests.ts            Adds jest-dom matchers to Vitest, and a stand-in ResizeObserver
vite.config.ts                Vite, the /api proxy and Vitest settings
```

## Navbar

`NavBar.svelte` is a bar across the top of the page, matching the website's at [umastable.com](https://umastable.com): the logo and "Uma Stable Match" on the left, linking back to the website, and on the right a round **?** button that starts the [product tour](#product-tour) (left out on the club page), then the theme switch. It stays at the top while scrolling, with a translucent, blurred background. The page's own heading, "Uma Stable Match club finder", is hidden visually and read by screen readers. On narrow screens a long title is cut short with an ellipsis rather than pushing the switch off the screen.

## Product tour

`Tour.svelte` walks visitors through the page, one part at a time. The steps are in `tour.ts`: a welcome; "Search by"; the trainer ID or pace field; balance scoring; the filters (full clubs, Discord, join style and playstyle); **Find clubs**; what the results show; "How to use"; the theme switch; and the **?** button.

- **When it shows.** It starts on a visitor's first visit. Finishing or skipping it sets `uma-stable-match:tour-seen` in local storage, so it doesn't start again on that device. A visitor who arrives with a search in the link sees their results first, and gets the tour on a later visit instead. If storage is blocked, the tour never starts on its own, rather than on every visit. The **?** button in the navbar starts it again at any time.
- **Highlighting.** Each step names the part it describes with a `data-tour` attribute, such as `data-tour="filters"` on the filter group in the form. The tour scrolls that part into view, dims the rest of the page with an SVG that has a rounded hole cut out around it, and draws an accent ring. Steps without a target, the welcome and the results, are centred. The card goes below the highlight if it fits, otherwise above it, and always stays on screen; it follows the highlight when the page scrolls or resizes. While the tour is open, the page underneath can't be clicked.
- **Controls.** **Next** and **Back** move between steps, **Skip tour** ends it, and the last step has **Done**. A row of dots shows progress.
- **Keyboard and screen readers.** The card is a modal dialog labelled by the step's title and described by its text. Focus moves to **Next** on each step and stays inside the card; Escape skips the tour, the left and right arrow keys step through it, and focus goes back to where it was afterwards.
- **Keeping it current.** The steps quote the page's labels, so update `tour.ts` when they change, and add a `data-tour` attribute for any new step's target.

## Search form

The form sits in a card. The "Search by" switch is at the top, then the field on the left and the options on the right, with **Find clubs** at the bottom right. Below 720 pixels it becomes a single column, and the inputs and button use the full width.

The form searches by **trainer ID** or by **daily pace**, with an optional **balance scoring** switch and strength slider (0 to 1, in steps of 0.05), an **include full clubs** switch, **join style** buttons and a **playstyle** dropdown.

- **Join style:** three pill buttons, Open, Approval and Closed, each with an icon. At most one is picked: picking another moves the choice, and picking the same one again clears it. The picked one takes its join style's colour, as on club cards. They're toggle buttons with `aria-pressed` in a labelled group.
- **Playstyle:** a dropdown of uma.moe's playstyles, with **Any playstyle** selected by default. It's `Dropdown.svelte` rather than a native `<select>`, so the open list matches the site in both themes. It follows the ARIA select-only combobox pattern: focus stays on the button, which announces the highlighted option with `aria-activedescendant`. Arrow keys, Page Up and Down, Home and End move the highlight; Enter, Space or Tab picks it; Escape or a click outside closes the list; typing jumps to the next option starting with those letters.
- **Layering:** the form card sits above the cards after it, so the open list isn't hidden behind "How to use".
- **Has Discord:** a switch that keeps only clubs whose description mentions Discord. While it's on, a note under the options says it's a best guess: some clubs with a Discord won't show up, and some links may have expired.
- **Warning:** while a join style or playstyle is picked, a note under the options says your best matches may be further from your pace.

| Field | Accepted | Sent to the API as |
|---|---|---|
| Trainer ID | 9 to 12 digits; spaces and dashes are ignored | `trainer_id` |
| Daily pace | A positive number of millions, such as `7.6` (a comma works as the decimal point) | `pace`, in fans per day |
| Balance scoring | Off, or a strength from 0 to 1 | `balance_weight`, only when on |
| Join style | One of open, approval and closed, or none | `join_style`, only when one is picked |
| Playstyle | One of uma.moe's playstyles, or any | `playstyle`, only when one is picked |
| Has Discord | On or off | `has_discord=true`, only when on |
| Include full clubs | On or off | `include_full=true`, only when on |

Only the field for the chosen mode is checked. An invalid field gets a message linked to it with `aria-describedby`, and nothing is sent. Links with an unknown or several join styles or playstyles open with none picked, as do older links with `include_closed=true`.

### How to use

Under the form, `HowToUse.svelte` is a collapsible "How to use" section, closed by default so it doesn't get in returning visitors' way. It walks through five steps: finding the trainer ID in the game (Menu → Profile), searching with it, searching by daily pace instead (with how to find it in [uma.moe's rankings](https://uma.moe/rankings), which opens in a new tab), the balance and closed-club options, and picking a club. A note says it works with the global (English) version and suggests searching by pace if a trainer ID isn't found. It quotes the form's and results' labels, so update it when those change.

### Search in the URL

Each search replaces the page URL's query with the same parameters the API uses, such as `?trainer_id=123456789012&balance_weight=0.5`. When the page loads with a valid search in the URL, it fills in the form and runs the search, so searches can be bookmarked and shared. Invalid or missing parameters are ignored. If a search is still running when a new one starts, the older one is cancelled.

### Remembering the trainer ID

"Remember my trainer ID on this device" is off by default. When it's ticked at search time, the trainer ID is saved in the browser's local storage under `uma-stable-match:trainer-id`. Unticking it and searching again deletes it. Nothing else is stored. Without a search in the URL, a remembered ID fills the form, but doesn't start a search on its own. Browsers that block storage simply don't remember.

## Search progress

While a search runs, `SearchStatus.svelte` shows "Finding clubs…" under a progress bar: a gold running figure moving left to right along a thin track, which fills in behind it. The figure is a sprite strip, `assets/runner/runner.webp`, played with a CSS `steps()` animation.

- **The curve.** The server doesn't report progress, so the bar is a guess based only on elapsed time: `progressAt()` in `progress.ts` eases towards 90% of the track, `0.9 × (1 − e^(−t/τ))` with τ of 4 seconds, and never reaches the end. A search that's taking long just leaves the figure near the end.
- **Per search.** `RunnerProgress.svelte` starts its own clock when it's shown and stops it when it's removed, so every search starts at the beginning and searches never affect each other. Nothing is sent to or read from the server.
- **Finish.** When a search succeeds, the figure dashes to the end of the track over a quarter of a second as the bar leaves. When it fails, or the visitor is told to wait, the bar goes at once.
- **Accessibility.** The bar is hidden from screen readers; the "Finding clubs…" text, in a `role="status"` region, is what they read. With reduced motion, the figure stands still on the first frame and doesn't dash, though the bar still fills.
- **Tuning.** `PROGRESS_TAU_MS` and `PROGRESS_CAP` in `progress.ts` set the speed and the point it stops at.

## Results view

After a search, the page shows a summary and the top ten picks.

- **Summary:** your pace, how many clubs were analysed (the API's `coverage.considered`: every club in the pool your options allow), and two stats about you from the returned clubs (`stats.ts`):
  - **Comfortable fits:** clubs where you'd sit 25% to 60% of the way down the members, the range fit aims for
  - **Typical place:** the middle of your place ÷ club size across the clubs, such as "Top 25%"

  Hovering over either value explains it. Under the stats, "Club data updated 3 hours ago." comes from the response's `updated_at`, with the exact time on hover. The balance setting is in the form.
- **Each card:**
  - **Header:** the pick number (gold, silver and bronze for the top three), the tier icon, the club name linking to uma.moe in a new tab
  - **Details:** the monthly rank and trend, open spots, join style and playstyle tags, a **Discord** tag when the API's `has_discord` is true, and the description
  - **How you'd compare:** your place and the club's median per day, with a **See how you fit** button under them that opens the [club page](#club-page)
  - **With balance on:** the balance score as a bar, passengers and top-three share. On wide screens they sit to the right of the description, so the card stays one row shorter
- **Notices:** under the summary, when they apply:
  - "*These results are filtered by join style or playstyle, so your best matches may be further from your pace than usual.*" when the API's `options` show a join style or playstyle filter
  - The Discord best-guess note, when `options.has_discord` is true
  - "*N more clubs are still loading. Search again in a minute for complete results.*" when the API's `coverage.pending` is above 0
  - "*These results are based on last month's data (February 2026), because the new game month has only just started.*" when `month` isn't the game month of `updated_at`
  - "*The game month has only just started, so these results combine this month's data with last month's. Last month's counts for less each day.*" when `updated_at` is fewer than `BLENDED_DAYS` (5) whole days into its game month, the days the server blends club paces with last month's
- **Top picks:** the first ten clubs with an open spot. Full clubs are only in the table. If every match is full, the page says so instead of "No clubs matched", and the table still shows them.
- **Notes:** under the top picks, always shown, even with no matches: full clubs aren't in the results (or, with **Include full clubs**, are only in the table), your own club never is, and a collapsed "Why not a higher-tier club?" explains that matches follow a club's typical member, not its tier. The note points to the **Your club** card when there is one, and otherwise suggests a trainer ID search. The FAQ on the marketing page answers the same two questions.
- **Your club:** when the response has `your_club`, a card between the notes and the table shows:
  - "Your club would be #37 in your results.", "fits you better than any of your results" at #1, or "would rank below your top 100" past the returned clubs, from `result_rank`
  - its median against your pace
  - a panel for your club and one for the top pick, when there is one, side by side on wide screens. Each has the tier, name, rank and trend, open spots and join style, a **See how you fit** button, then the median, its gap to your pace (`+3.6M/day` above it, `−1.2M/day` below), your place and, when the club has them, the balance details
- **Layout:** on narrow screens, up to 720 pixels wide, the numbers move below the club details, the description and balance details stack, and the **Your club** panels stack.
- **Screen readers:** after a search, only a short "Found N clubs" message is announced, followed by any notices, not the whole list. The text goes into a live region that's always on the page, because a notice that appears together with new results isn't reliably announced. Rank trends have spoken descriptions, such as "Up 16 places since last month".

### Tier icons

Tiers are shown with the game's own icons, D to SS, from [`src/assets/tiers/`](src/assets/tiers/README.md). They're © Cygames, Inc. and excluded from the MIT licence; that folder's README has the details. Each icon is under Vite's 4 kB inline limit, so it's bundled into the JavaScript as a data URL. A content security policy for the page must allow `img-src data:`.

### Top results table

Below the cards, the 100 best-fitting clubs are listed in a table, best fit first, under "Top N results" with the number returned, so it's clear these are the best matches rather than everything searched. The app asks the API for 100 clubs (`RESULT_LIMIT` in `App.svelte`), as clubs further down fit poorly. Searching and sorting work within those 100, while **Clubs analysed** in the summary counts every club compared with your pace. Each club name links to uma.moe, with a small **See how you fit** button beside it.

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

- **Sorting:** select a column header to sort by it. Select it again to reverse the order. Missing values always sort last, and ties keep pick order. Each header is a button with `aria-sort`, so sorting works by keyboard and screen readers announce it.
- **Search:** filters by club name as you type. "Showing X of Y clubs" updates as you go.
- **Hide closed clubs:** only appears when the search included closed clubs, and is on by default.
- **Full clubs:** with **Include full clubs**, their Open spots cell shows a red "Full" tag. The `#` column still counts them, so it can differ from the top-pick numbers.
- **Narrow screens:** the table scrolls sideways inside its own region. The region can be focused, so it also scrolls with the keyboard. That needs a `svelte-ignore` for Svelte's `a11y_no_noninteractive_tabindex` rule, because axe requires scrollable regions to be focusable. On a desktop the table fits without scrolling, even with balance columns.

### Themes

Colours are CSS custom properties in `src/app.css`, and components only use these tokens. Each token holds both values, as `light-dark(light, dark)`, and the root's `color-scheme` decides which one applies:

| `<html>` | `color-scheme` | Result |
|---|---|---|
| No `data-theme` | `light dark` | Follows the system setting |
| `data-theme="light"` | `light` | Always light |
| `data-theme="dark"` | `dark` | Always dark |

`light-dark()` needs Chrome or Edge 123, Firefox 120 or Safari 17.5, or newer.

The **System / Light / Dark** switch in the navbar (`ThemeToggle.svelte`) sets the attribute. It saves Light or Dark in local storage under `uma-stable-match:theme`, and choosing System deletes it. It's a group of radio buttons labelled "Theme", so arrow keys and screen readers work, and keyboard focus shows a ring. Each choice is an icon (a monitor, a sun and a moon) with its name hidden for screen readers, and a themed tooltip shows the name on hover or keyboard focus.

The website at umastable.com has the same switch and palette, with the same storage key. Browsers keep storage per site, though, so a choice made on one isn't carried over to the other.

`public/theme-init.js` applies a saved theme before the page renders, so it never flashes the wrong theme while the app loads. `index.html` loads it from its own file because the server's content security policy blocks inline scripts. Vite copies it into `dist/` unchanged, so it isn't hashed and browsers revalidate it on each visit.

The palette follows the project's logo and website: navy with a gold accent in the dark theme, and cool white with navy text and a deeper gold in the light theme, so gold text still meets the contrast check. The status colours for rank changes, join styles and the top three picks are the same in both. `index.html` sets a matching `theme-color` for each, which colours the browser bar on phones.

### Brand artwork

The logo and the background artwork each have a light and a dark version in `src/assets/brand/`, made from the originals in [`docs/images/`](../docs/images/): `logo.jpeg`, `hero.jpeg` and their `-light` versions.

- **Logo.** `BrandLogo.svelte` shows the logo in the navbar and the footer. It renders both versions, and the `.only-light` and `.only-dark` classes in `app.css` show the one for the current theme, whether it comes from the system or the theme switch.
- **Background.** The search form and "How to use" sit in a `.hero` section. Its `::before` shows the wide artwork (`--hero`), dimmed by `--hero-opacity` and faded to transparent towards its edges with a radial mask, so no box shows whatever the image's background colour. It's anchored to the top of the section, and up to 720 pixels wide to the bottom, so on phones it sits behind the form rather than the title. It stays inside the section, so it never shows behind results or the footer.
- **Search form.** The form's card is translucent with a slight blur, so the artwork shows through it.
- **Alignment.** The light and dark artwork are aligned so their outlines lie on top of each other. Keep them aligned when replacing either: measure the drawing in both, and scale and centre the light version to match.
- **Icons.** The language button's icon is "translate" from [Bootstrap Icons](https://icons.getbootstrap.com/) (MIT licence). `public/favicon-32.png` and `public/apple-touch-icon.png` are the dark logo at 32 and 180 pixels.

### Page width

`<main>` and the footer share two tokens from `app.css`, so their edges always line up:

- `--page-max` is `1760px`. The page fills the screen up to that width, then stays centred.
- `--gutter` is the side padding: `16px` up to 720 pixels wide, `24px` above that, and `32px` from 1600 pixels.

Card descriptions are capped at `80ch` so they stay easy to read on wide screens.

### Shared styles

`src/app.css` also styles the controls used across the page, so components don't repeat them:

| Style | Use |
|---|---|
| `button` | Every button: a card-coloured background, a border and a hover state. Disabled buttons are dimmed |
| `button.primary` | The main action, such as **Find clubs**: accent background with `--on-accent` text |
| Text and search inputs | Padding, border and theme colours. `aria-invalid="true"` turns the border red |
| `accent-color` | Checkboxes, radio buttons and the slider follow the theme's accent |
| `.check` | A checkbox and its label, aligned on one line |
| `.switch` | A checkbox drawn as an on/off switch: a label holding the input (`role="switch"`), a `.track` span, then the text. Gold when on. Used for balance scoring, full clubs, Has Discord and remembering the trainer ID |
| `.tooltip` | A themed tooltip under the element, from its `data-tooltip` text, on hover and keyboard focus. Used for the theme icons and the tour button, which also have accessible names |
| `.segmented`, `.segmented.small` | A pill-shaped group of radio buttons, used for "Search by" and, with icons, the theme switch. The selected option is styled with `:has(input:checked)` |
| `:focus-visible` | A 2px accent ring around whatever has keyboard focus |

### Footer

`SiteFooter.svelte` is the page's `<footer>` landmark, below `<main>`.

| Part | Content |
|---|---|
| Tips for switching clubs | Collapsible. The four tips from the root README: the 14-day rule, the 24-hour cooldown, the month-start lock, and checking activity first |
| Brand | The logo and the project's name |
| Report a problem | A link to the issue tracker |
| Buy me a carrot | A gold link to the project's Ko-fi page, with a note that tips only help cover hosting and every feature stays free. It's a plain link, as the content security policy blocks Ko-fi's widget script |
| Disclaimer | An unofficial, non-commercial fan tool, not affiliated with uma.moe or Cygames. uma.moe is credited as the data source, and the game and tier icons as © Cygames, Inc., with a link to Cygames' Fan-Created Content Guidelines |

The tips sit in a card, and the rest is centred below it. External links open in a new tab with `rel="noopener noreferrer"`.

The disclaimer follows the root README's Disclaimer section; change both together.

### Accessibility

The page aims for WCAG 2.2 AA. Two test files check it, and they run with the rest of the tests in CI.

**`contrast.test.ts`** reads the colour tokens straight from `app.css` and checks every pair the UI uses, in both themes. Tinted tag backgrounds and semi-transparent colours are blended over the card colour first.

| Kind | Minimum | Examples |
|---|---|---|
| Text | 4.5:1 | Body, secondary and muted text on the page, cards and surfaces; links; button text; trend, error and warning text; tag text on its tint |
| Large text | 3:1 | Gold, silver and bronze pick numbers |
| UI parts | 3:1 | Form field borders (`--control-border`), and the focus ring on surfaces |

When you add or change a colour token, add its pairs to the test. A failing pair names the tokens involved.

**`a11y.test.ts`** runs [axe-core](https://github.com/dequelabs/axe-core) over:
- the whole page: empty, with results and the table, and after a failed search
- the form, with a validation error and with balance on
- the sorted table
- every search status
- the theme switch
- the open "How to use" section
- the whole page with the product tour open

Two axe rules are adjusted. The colour-contrast rule is off, because jsdom doesn't render, and the contrast test covers it instead. The landmark (region) rule only applies to the whole-page checks.

**Keyboard:** everything works without a mouse.
- Tab moves through the tour button, the theme switch, "Search by", the form, the cards, the table search, the scrollable table and its column headers.
- The arrow keys change radio groups, Enter submits a search, and Enter or Space sorts a column.
- `:focus-visible` shows a ring on whatever has focus.

### Search states

`src/lib/runner.ts` runs each search and tracks where it's up to. `SearchStatus.svelte` shows that state under the form.

| Situation | What the visitor sees | Retry |
|---|---|---|
| Loading | "Finding clubs…". Previous results stay, dimmed and marked `aria-busy` | – |
| First scan still listing clubs (503) | "Club data is still loading", with a countdown | Automatic after 5, 10, 20, 30, 30 and 30 seconds (about 2 minutes), or **Try now** |
| Still loading after every retry | "Club data still isn't ready" | **Try again** |
| Rate limited (429) | "Too many searches", counting down from `Retry-After` (10 seconds without it) | **Try again**, enabled when the countdown ends. Never automatic |
| Trainer not found (404) | Where to find the trainer ID in-game | None; the search needs changing |
| Not enough data (422) | Wait a few days, or search by pace | None |
| uma.moe unreachable (502), network error, timeout, other | A short explanation | **Try again** |
| Invalid search (400) | The server's message | None |
| No matches | "No clubs matched your search", with hints to turn off Has Discord, set playstyle back to any, pick Closed under join style, or lower the balance strength, when those apply | – |

Starting a new search cancels any pending retry and any earlier request, so an old answer never replaces a newer one. Loading and waiting messages use `role="status"`, and errors use `role="alert"`. The dimming transition is turned off when the visitor prefers reduced motion.

The runner takes the fetch function, a change callback, the retry delays and the timer functions as options. Its tests replace them, so they can check retries and countdowns to the second with fake timers.

## Club page

`/club/{id}` shows one club, and `/club/{id}?trainer={trainer ID}` places that trainer among its members. `main.ts` mounts `ClubPage.svelte` for these addresses and the search for everything else; the server answers any page address with `index.html`. A trainer ID that isn't 9 to 12 digits is dropped, so the club still shows. `clubPath()` builds the address.

The page calls `fetchClub()`, which wraps [`GET /api/v1/clubs/{id}`](../backend/server/README.md#club-page) and uses the [human check](#human-check) like a search.

- **Intro:** one line above the header saying what the page is for. With a trainer: see how you'd fit before you apply, your rank among the members and this month's daily gains. Without one, it describes the club and suggests a trainer ID search.
- **Header:** the tier icon, the name linking to the club's uma.moe page, the same rank, spots and tags line as the cards (`ClubTags.svelte`), the description, and the median. With a trainer, their pace and "You'd be 6 of 30" too.
- **Chart:** right under the header, `PaceChart.svelte` draws each member's line thin and muted, the club median in bold and the trainer's line in the accent colour, with an end dot. It shows **Daily gain** by default, and **Fans this month** shows the fans gained since each line's first day. A legend names the lines; see [Pace chart](#pace-chart).
- **Members:** a numbered list of cards under the chart, one per current member, fastest first, filling as many columns as fit. Each card shows the place, trainer name and daily pace. A member without a name shows as "Unknown trainer". With a trainer, each card also says how far its pace is from theirs, such as "+2.5M/day vs you". Members too new to rank come last, without a place.
- **Highlighting a member:** each member card is a toggle button. Selecting it highlights the card and the member's line, scrolling the chart into view if it's off screen; selecting it again, or another card, clears or moves the highlight.
- **You:** `rosterRows()` adds the trainer as a highlighted card after every member faster than them, the same rule the server uses for "You'd be". In their own club, their own entry is left out and a note says they're placed as if joining.
- **Loading:** "Loading the club…" under the same running progress bar as a [search](#search-progress).
- **Errors:** a club that isn't loaded yet says so and offers a retry. Other errors read as they do for a search.
- **See how you fit:** every top pick card, both **Your club** panels and every table row have a gold outlined button to the club page, opening in the same tab. On cards it sits under "You'd be" and the median; in the table it's a smaller version beside the name. After a trainer ID search it reads **See how you fit** and carries the trainer, so the page places them. After a pace search the page can't place anyone, so it reads **See members** and shows the club alone. Each button's spoken name includes the club, such as "See how you fit in Example Club".
- **Back to search:** following one of those buttons notes the search's address in this tab's session storage. The club page's **Back to search** then goes back in history to those results, with their scroll position. Without one, for example when the club page was opened from a shared link, it opens a new search. The server sends `Referrer-Policy: no-referrer`, so the referrer can't tell where the visitor came from.

The navbar keeps the release notes but has no tour button here.

### Pace chart

`chart.ts` turns each member's cumulative fans into what the chart plots:

- **Daily gain:** each day's gain over the day before. After days without data, the gain is spread evenly over them. The first day has no gain, so the daily chart starts on day 2.
- **Fans this month:** each day's fans minus the first day with data.
- **Club median:** the median of the members' values on each day, drawn where at least 3 members have one. It's a day-by-day median, so it can differ from the blended **Median/day** in the header.

- **The day under way:** uma.moe's daily update already includes the current day, so its gain is only part of a day and would show as a sharp drop. While the data is for the current game month (or has no month), the chart stops at the day before. Once a month is over, every day shows.

Days are numbered from 1, with an axis label every fifth day. The y axis starts at 0 and steps through round values such as 2.5M. The chart is plain SVG sized to its container, with no chart library.

- **Reading values:** a crosshair follows the pointer and snaps to the nearest day, and a readout lists the highlighted member's value, yours, the median and the members' range that day. The chart is a slider over the days: the arrow keys, Home and End move it, and screen readers hear the same values as the readout.
- **Highlighted member:** their line is drawn on top in `--highlight`, with an end dot, and named in the legend.
- **Colours:** members use `--muted` at 35% opacity, the median `--text`, you `--accent` and the highlighted member `--highlight`. Those three lines stay apart for colour-blind readers in both themes.

## Display formatting

`src/lib/format.ts` turns API values into display text. The functions are pure; `formatUpdated` takes the current time as a parameter, so tests can fix it.

| Function | Example |
|---|---|
| `formatMillions` / `formatPerDay` | `6400000` → `6.4M` / `6.4M/day`; `850000` → `0.85M` |
| `formatPace` | `850000` → `850K/day`; `420` → `420/day`; from 1M as `formatPerDay` |
| `formatGap` | `3600000` → `+3.6M/day`; `-850000` → `−850K/day` |
| `formatPlace` | `8, 30` → `8 of 30` |
| `formatPercent` | `0.344` → `34%` |
| `rankTrend` | `111, 127` → `▲16` ("Up 16 places since last month"); no last-month rank → `new` |
| `joinStyleLabel` | `approval` → `Approval` |
| `playstyleLabel` | `2` → `Laid-back`, using uma.moe's names for policy codes 1 to 17 |
| `formatUpdated` | 3 hours earlier → `3 hours ago`; a day earlier → `yesterday` |
| `gameMonth` | `2026-03-01T15:00:00Z` (the 2nd in Japan) → `2026-03`; `2026-03-01T14:59:59Z` → `2026-02` |
| `daysIntoGameMonth` | `2026-03-01T15:00:00Z` → `0`; `2026-03-06T15:00:00Z` → `5` |
| `monthLabel` | `2026-02` → `February 2026` |

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
| `human-check` | The human check couldn't be completed | The server's status, or `null` | A short message to show |

`retryAfter` holds the response's `Retry-After` header in seconds, or `null` without one. The server sends it with `429` responses.

### Human check

`src/lib/human.ts` gets the token the server asks for when its [human check](../backend/server/README.md#human-check) is on. The first search asks `GET /api/v1/challenge` for Turnstile's site key. A `404` means the check is off, and the page never asks again or loads anything from Cloudflare.

- **Challenge:** the page loads Turnstile's script from `challenges.cloudflare.com` only then. The widget stays hidden unless the visitor has to interact. It's shown in a `.human-check` box at the bottom of the page and removed afterwards.
- **Token:** the response is traded at `POST /api/v1/challenge` for a token, kept in memory (never stored) and sent to `/api/v1/recommendations` as `Authorization: Bearer`. A new one is fetched 30 seconds before it expires.
- **Retry:** a `401` clears the token, fetches a new one and repeats the request once. `fetchClub` does the same.
- **Failure:** a challenge that fails, times out after 60 seconds or can't load shows "We couldn't confirm you're a person", with a retry.
- **Needs the CSP:** the page can only load the script and widget if the server's content security policy allows `challenges.cloudflare.com`.

## Tooling

- **TypeScript** runs in strict mode, with `noUncheckedIndexedAccess`.
- **ESLint** uses flat config, with the recommended JavaScript, TypeScript and Svelte rules.
- **Prettier** uses single quotes and 100-character lines. Markdown files are left alone.
- **Tests** use Vitest with jsdom and [Testing Library](https://testing-library.com/docs/svelte-testing-library/intro). Stub `fetch` with `vi.stubGlobal` rather than calling a real server.

## End-to-end test

`e2e/` holds a [Playwright](https://playwright.dev) test that drives the built page in Chromium against the real server: it searches by daily pace and checks for top picks and the results table. It catches broken wiring between the page, the human check and the server that the component tests, with their stubbed `fetch`, can't.

- **Server.** `playwright.config.ts` builds the page, then starts the server's [`fixture_server` example](../backend/server/README.md#fixture-clubs) on port 8181, serving `dist/` with made-up clubs instead of uma.moe. A cold build of the server takes a few minutes.
- **Human check.** The server uses Cloudflare's always-pass Turnstile test keys, so the check runs for real without anything to solve. Turnstile's script and token check still call Cloudflare.
- **Running it.** It needs Rust as well as Node. Install the browser once with `npx playwright install chromium`, then run `npm run e2e`. Locally, a server already running on port 8181 is reused.
- **Failures.** A failed test keeps a trace in `test-results/`; open it with `npx playwright show-trace <path to trace.zip>`.
- **In CI.** After CI passes on `main`, the Deploy workflow runs the test, and deploys only if it passes. Pull requests don't run it. A failed run uploads the HTML report and traces as the `e2e-report` artifact. It also comments on the merged pull request and opens an issue labelled `e2e-failure`, or comments on the one already open. Close that issue once the test passes again.

## Analytics

Analytics are off unless the server's `WEB_ANALYTICS_TOKEN` is set. Both the search and the club page read the token from `GET /api/v1/features` at startup, through `applyFeatures()` in `src/lib/features.ts`, and add Cloudflare's Web Analytics beacon (`src/lib/analytics.ts`), which counts page views without cookies. If the request fails, the beacon isn't loaded.

## Language

The Japanese translation is behind the server's `ENABLE_JAPANESE` setting, off by default. At startup both the search and the club page ask `GET /api/v1/features`, through the same `applyFeatures()`; if the answer is missing or fails, they stay in English. With it on, a translate button appears in the top bar and the saved choice (`en` or `ja`) is shown, and a visitor with no saved choice gets English. The choice is saved in a `uma-stable-match-language` cookie on `umastable.com`, so it carries over to the website and back, and in local storage under `uma-stable-match:language`. The cookie wins when both are set; local storage covers choices made before the cookie existed, and hosts such as `localhost`, where the cookie stays on that host.

### Messages

Every piece of text the app shows is a message in `src/lib/messages/en.ts`, looked up with `t('key')`, or `t('key', { n })` for a message with values. A message is a string, or a function when it has numbers or plurals. Call `t()` where the text is shown, never in a module-level constant, so it follows a language switch. A few messages hold markup (the how-to steps and the footer) and are shown with `{@html}`; they are first-party text, so never put visitor input in one. Add every new message to both `en.ts` and `ja.ts`; the types and a test enforce it. In Japanese, use the game's own terms (a club is a サークル, never "club"); a test fails on the English word.

## Release notes

The sparkle button in the navbar opens the release notes. It carries a dot until the visitor has opened them at the newest build. The notes are in English and Japanese, like the rest of the app.

Publish a note only for a change a visitor would notice: a new feature, a better result or a fixed bug. Skip refactors, tests, tooling, docs, running locally and anything else that doesn't change what the app does for them. Not every merged pull request needs one.

### Build numbers

Builds are `major.minor.patch`:

| Raise | When | Release kind |
|---|---|---|
| minor | A new feature or a visible improvement | `update` |
| patch | A hotfix that only fixes a bug | `hotfix` |
| major | A redesign or a change that breaks old links | `update` |

The site launched as `1.0.0` on 23 October 2026. A release's date is the day it went live, in `YYYY-MM-DD`. Group the changes that go out together under one build.

### Add a release

1. Add the release at the **top** of `RELEASES` in `src/lib/releases.ts`, with its build, date, kind and changes. Each change has a kind (`new`, `improved` or `fixed`) and the keys of its title and body messages.
2. Write both messages in `messages/en.ts` and `messages/ja.ts`, under `releases.<name>.title` and `releases.<name>.body`. The tests fail if one language is missing.

Writing the notes:

- Write for players, not developers: say what changed for them and why it helps. No issue or pull request numbers, no code, no setup or hosting details.
- Use the labels players see in the app (such as **Include full clubs**), and the game's own terms in Japanese.
- Title as a short phrase without a full stop. Body as one sentence that starts with what the player can now do, such as "Track goals, compare results with your plan, and carry savings forward." Don't narrate the fix or add caveats.
- A hotfix lists only `fixed` changes. Start each title with "Fixed" and say in plain words what was wrong, such as "Searching by trainer ID no longer says your trainer wasn't found."
- Never edit the notes of a published release except to fix a mistake. Describe later changes in a new release.
