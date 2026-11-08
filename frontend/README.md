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
src/main.ts                   Mounts the search, the club page for a /club/ address, a club applications page or the settings page
src/App.svelte                Root component: runs searches and keeps them in the URL
src/lib/route.ts              Which page an address shows, and club, wishlist, club applications and settings page addresses
src/lib/ApplicationsPage.svelte    The club applications page for Discord session sign-in and role selection
src/lib/ClubApplicationsPage.svelte A club's page for its leaders: adding the bot, then its status and applications
src/lib/BotSetup.svelte       The steps to add the bot, ticked as they're done
src/lib/BotPanel.svelte       The bot's status, the club's settings and removing the bot
src/lib/ApplicationsTable.svelte   A club's applications, filtered by status, with ApplicationDetails.svelte for one
src/lib/TrainerApplicationsPage.svelte A trainer's own page: the applications they've sent in the last month
src/lib/TrainerApplicationsTable.svelte A trainer's active applications, filtered by status
src/lib/applications.ts            The applications API client: sign-in, the leader's club, the invite check, settings, unlink, trainer links, deleting the account and application expiry
src/lib/SettingsPage.svelte   The settings page: link a trainer ID, sign out, delete the account
src/lib/AccountMenu.svelte    The navbar's account menu for a person signed in with Discord
src/lib/account.svelte.ts     Who is signed in, shared by every component
src/lib/session.ts            Remembering who is signed in between pages: the signed-in cookie and the saved person
src/lib/WishlistPage.svelte   The wishlist page: StrawPoll polls for ideas, in frames
src/lib/polls.ts              The polls on the wishlist page
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
src/lib/NavBar.svelte         Top bar: the brand, linking to the website, the pages pill, the release notes and tour buttons, the theme switch and the account menu
src/lib/NavLinks.svelte       The pages as a pill of icon links in the navbar, icons only on narrow screens
src/lib/pages.ts              The pages the navbar links to, and which one is showing
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
src/lib/SiteFooter.svelte     Tips, the logo, buttons to report a problem and support the project, the disclaimer, and Privacy Policy and Terms of Service links once applications are on
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

`NavBar.svelte` is a bar across the top of the page, matching the website's at [umastable.com](https://umastable.com): the logo and "Uma Stable Match" on the left, linking back to the website (`site.localhost` on the same port when the app runs on `localhost` or a name under it, as when the [server serves both](../backend/server/README.md#serving-the-website)), and on the right a round **?** button that starts the [product tour](#product-tour) (left out on the club page), then the theme switch and, for a person signed in with Discord, the [account menu](#account-menu) at the far right. It stays at the top while scrolling, with a translucent, blurred background. The page's own heading, "Uma Stable Match club finder", is hidden visually and read by screen readers. On narrow screens a long title is cut short with an ellipsis rather than pushing the switch off the screen.

### Account menu

`main.ts` works out who is signed in once per page (`loadAccount()` in `account.svelte.ts`), waiting for it only on pages that need a session. Pages don't ask again themselves: anything that needs the person reads `account` or calls `loadAccount()`. It asks the server as little as it can, through `session.ts`:

- Without the server's `uma_signed_in` cookie, nobody is signed in and the server isn't asked. The server sets that cookie beside the session, readable by the page, holding the session's expiry, and clears it on sign-out and account deletion. It expires with the session.
- With it, the person saved in local storage (`uma-stable-match:account`) for the same session is reused. Only after signing in, or when nothing is saved, does the page ask `GET /api/v1/auth/session`, and it saves the answer.
- Any 401 from the applications API, signing out and deleting the account drop both, so a session ended elsewhere stops counting at the next request. Signing out in one tab signs the others out through the `storage` event.

The saved person only chooses what the navbar shows and whether a page sends you to sign in; every API call still checks the session. After deleting the account on one device, another may keep showing the account menu until its next request.

 Once someone is, `AccountMenu.svelte` shows last in the navbar: a pill with their Discord name (or "Discord account" when the server couldn't look it up), cut short with an ellipsis when long. It opens a menu with **Signed in as …**, **Settings** and **Sign Out**, following the ARIA menu button pattern: arrow keys, Home and End move between the items, and Escape, Tab or a click elsewhere closes it. Signing out from a page that needs a session goes to the club finder; elsewhere the menu gives way to the sign-in button.

Signed out, the same place holds a round silhouette button, **Sign in**, linking to `/login?next=<this page>`. It waits until the account is worked out (`account.known`), so a signed-in person never sees it flash first, and it's left out on the login page itself.

The menu or the sign-in button makes the tools wider, so while either shows (`showsAccount()`) the page names in the pill show from 1440px rather than 900px, and the three-column layout starts at 1100px rather than 960px. Below that the navbar wraps, as it does on phones.

## Settings page

`/settings` (`SettingsPage.svelte`) is for a person signed in with Discord; `main.ts` sends anyone else to `/login?next=/settings`. It isn't one of the navbar's pages, so none is marked current there; the [account menu](#account-menu) links to it. It has three cards:

1. **Link your umamusume account.** The person types a trainer ID (9 to 12 digits; spaces and dashes are dropped) and presses **Link Account**. The server gives a key such as `uma-7KQ4XMPR`, shown with **Copy key** and how to add it to the trainer's profile comment in the game. **Check** asks the server to look for it. Found at once, the trainer joins the list below. Otherwise the server keeps looking every minute for 5 minutes, and the button turns grey with a spinner and reads **Verifying** until it ends. The page only asks the server how it's going (every 5 seconds, from `GET /api/v1/discord/trainers`), so it never calls uma.moe itself. Reloading or coming back later picks up the newest pending key and any running check. **Cancel** forgets the key on the server, which stops its check. A person links one trainer at a time: once one is linked, the form gives way to a note to unlink it first. The linked trainer shows its name (when kept) and ID, with **Unlink**. That trainer is how the club's applications page knows which club the person leads. The page shows the server's other answers as messages: not found after 5 minutes, already linked, unknown trainer, too many trainers, too many tries, and an expired key, which starts over. See [trainer links](../backend/discord/README.md#club-links) for the endpoints. If the server doesn't offer trainer links, the card says so instead.
2. **Sign Out** ends the session and goes to the club finder.
3. **Delete Account** asks once, then deletes the person's trainer links and pending keys and signs them out (`DELETE /api/v1/auth/account`). Clubs they added the bot to keep it.

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

`SiteFooter.svelte` is the page's `<footer>` landmark, below `<main>`. Its last line is **Privacy Policy** and **Terms of Service** links to the website's `/privacy/` and `/terms/` pages, in new tabs.

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

### Quick apply badge

`ClubTags.svelte` adds a **Quick apply** badge when a club's `applications` is `"discord"`: on result cards, table rows and the club page. It is separate from the **Discord** tag, which only says the description mentions Discord. The server sets `applications` for clubs that are opted in, open, joinable and without a setup problem, so the badge needs no logic here, and the field is left out when Discord isn't set up. The badge uses the Discord tag's colours, so the same [contrast](#accessibility) pair covers it.

## Club applications pages

**Placeholder copy.** All text on these pages is lorem ipsum for now: the `applications.*` messages (except `applications.link`, the navbar label), in both languages. The website's privacy page is real, English-only text. Tests read the text through `t()`, so replacing it needs no test changes.

`/applications`, `/applications/club` and `/applications/club/{id}` are where a club leader sets up applications through Discord. `main.ts` mounts `ApplicationsPage.svelte` or `ClubApplicationsPage.svelte`; `applicationsRoute()` reads the address and `clubApplicationsPath()` builds it. A **Club applications** link joins the navbar's pages. `applications.ts` wraps the [Discord endpoints](../backend/discord/README.md); they aren't behind the human check, and they use the session cookie, so requests are same-origin.

**The page at `/applications`** says what taking applications through Discord does and what it needs, in the wording written for club leaders: a working invite in the club description while opting in, and Manage Server or Administrator in the server. It has no sign-out button or privacy link of its own: signing out is in the account menu and the settings page, and the privacy link is in the footer and on the sign-in page. Then:

1. **Sign in:** only the sign-in page (`LoginPage.svelte`) offers **Sign in with Discord**, which goes through `/api/v1/auth/discord/login` and comes back here. The page links to the website's privacy page beneath its card.
2. **Result:** Discord sends the leader to the club's applications page with `?linked=1`. If something fails in the callback, the server sends the browser back to `/applications?error=<label>`, and the page shows what each label means (`wrong_server`, `no_server`, `not_manager`, `not_leader`, `denied`, `invalid_state`, `discord_error`, `unavailable`). An unknown label reads as a generic problem, and only a short lowercase label is accepted from the address.

**The applicant comparison** (`ClubComparePage.svelte`) is at `/applications/club/{id}/compare?trainers=a,b`, opened by **Compare in club** in an application's details. It's for the club's leader only: the page checks `fetchMyClub()` says they lead club `{id}` and otherwise shows the same sign-in and forbidden messages as the applications page. It calls `fetchClubCompare()`, which wraps [`GET /api/v1/clubs/{id}/compare`](../backend/server/README.md#comparing-trainers) with the [human check](#human-check), and shows the club's chart (`PaceChart`'s `lines`: one coloured line per trainer, up to 5) and, below it, one list of the members and the compared trainers, ranked together (`compareRows()`): a trainer's card is placed after everyone faster, shows their live pace, the gap to the club median and a remove button, and takes a place in the numbering like a member. (The server's `your_place` is each trainer among the members alone; the page doesn't show it.) It starts with the trainers in the address, normally one applicant. The **Fan requirement** form takes an amount in millions and an interval (day, week or month) and draws it on the chart as a dashed line (`requirementPerDay`, from `perDay()` in `requirement.ts`): flat in the daily view, and rising by the daily amount each day in the monthly one. A week is 7 days and a month is the game month's days. It's kept in the address as `need` (fans) and `per`, and in this browser's `localStorage` for each club (`rememberRequirement()`), so a visit with nothing in the address starts from it; the address wins when it has one, and **Clear** forgets it. With a requirement set, each member's and compared trainer's card says whether they meet it and by how much. The chart's axes are labelled with what they measure. The leader adds more from the club's active applications or by trainer ID, up to 5 (`MAX_COMPARED`), and removes any but the last; the address follows with `history.replaceState`. The club's size doesn't matter, so nothing is hidden when the club is full.

**The club's applications page** is for the club's leader in the game, and no one else. The **I'm a club leader** card opens `/applications/club`, and the page finds the club itself: `GET /api/v1/discord/my-club` (`fetchMyClub()`) gives the person's linked trainer, the club it's in and whether it leads it. For a leader, the address changes to `/applications/club/{id}` for that club (an address naming another club changes too), and the page loads `GET /api/v1/discord/clubs/{id}/settings`, which the server checks against the club's leader on every request. It shows what it's told: **sign in** for a 401 and **only the club's leader can manage it** for a 403. The heading is the club's name with a status chip: **Not set up**, **Taking applications**, **Paused** or **Needs attention** (the server reported a setup problem).

- **Before the bot is added** (a 404, no trainer linked, or a trainer that leads no club), `BotSetup.svelte` explains how applications work and lists four steps with round checkboxes. The first two can be checked from here:
  1. **Link your Umamusume account** is ticked when a trainer is linked. Until then, **Open Settings** goes to `/settings`. A linked trainer that doesn't lead a club gets a warning, and the steps after it wait.
  2. **The invite in the club description** says it's how we know the server belongs to the club. For a leader, `hasInvite()` calls the opt-in address without following its redirect, so a redirect means the description has a working invite to one server and a 422 means it doesn't. Until one is found, **Add the bot to my server** stays disabled and **Check again** repeats the check. If the check fails for another reason the button works anyway, since the opt-in checks the invite itself.
  3. **Manage Server or Administrator** and 4. **adding the bot** are done once Discord sends the leader back.
- **Once it's added**, `BotPanel.svelte` sits beside the applications: the bot's status as a checklist, the settings (**Take applications**, **Add applicants to the server**) and **Remove the bot**, which asks once before sending the `DELETE` and then shows the setup steps again. When applications are paused, a notice above the table has a **Resume** button.
- **Applications.** `ApplicationsTable.svelte` lists them newest first with status filters, as cards on narrow screens, and **View** opens `ApplicationDetails.svelte`, laid out like the message the bot posted, with a link to the ticket channel. No endpoint lists a club's applications yet, so the page passes an empty list and the table shows its empty state.

**A trainer's applications page** (`/applications/trainer`, `TRAINER_APPLICATIONS_PATH`) is where a trainer follows the applications they've sent. The **I'm a trainer** card on `/applications` opens it. Like the club's page, it needs a session, so `main.ts` sends a signed-out visitor to `/login` first. It has no bot setup or bot panel: just `TrainerApplicationsTable.svelte`, which lists each application's club (linking to its club page), trainer ID, submitted and expiry dates, status and ticket link, newest first with the same status filters, and as cards on narrow screens.

An application expires one calendar month after it was sent, or on the last day of that month when it has no such date (31 January expires on 28 February). `expiresAt()` in `applications.ts` works this out in UTC, and `activeApplications()` drops the expired ones, so the table only ever shows active applications. No endpoint lists a trainer's applications yet, so the page passes an empty list and the table shows its empty state.

### Pace chart

`chart.ts` turns each member's cumulative fans into what the chart plots:

- **Daily gain:** each day's gain over the day before. After days without data, the gain is spread evenly over them. The first day has no gain, so the daily chart starts on day 2.
- **Fans this month:** each day's fans minus the first day with data.
- **Club median:** the median of the members' values on each day, drawn where at least 3 members have one. It's a day-by-day median, so it can differ from the blended **Median/day** in the header.

- **Early in the month:** while this month has fewer than 4 days of data, the fewest the daily chart needs, the server also sends the club's `previous` month, and the chart plots that instead: every member's line from last month, lined up with the member cards, plus your own. Members who joined this month have no line. Highlighting a member still works, and every day shows, since last month is over. From 4 days of data the server stops sending it and the chart shows this month.
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

`src/lib/human.ts` gets the token the server asks for when its [human check](../backend/server/README.md#human-check) is on.

- **Warm-up:** the search page starts the check as soon as it loads, unless one passed lately, so it has usually passed by the time the visitor presses **Find clubs**, even with a pasted trainer ID. A failure is left for the search to retry. The club page needs no warm-up: it loads the club as it opens, which runs the check if one is needed.
- **Skipping a check:** a passed check stores only its expiry in local storage (`uma-stable-match:human-until`), as a sign that the [token cookie](../backend/server/README.md#token-cookie), which scripts can't read, is likely still valid in any page or tab. While that holds, the page loads nothing from Cloudflare and requests go out with the cookie alone.
- **No probe:** without that sign, a request waits for the check rather than going out to learn of it from a `401`. With the check off, the one `GET /api/v1/challenge` answers `404` and the check isn't asked about again.
- **Challenge:** the page asks `GET /api/v1/challenge` for Turnstile's site key and loads Turnstile's script from `challenges.cloudflare.com`. The widget stays hidden unless the visitor has to interact. It's shown in a `.human-check` box at the bottom of the page, above the tour and release notes, and removed afterwards.
- **Token:** the response is traded at `POST /api/v1/challenge` for a token, which the server also sets as the cookie. The page keeps it in memory too (never in storage) and sends it as `Authorization: Bearer` until 30 seconds before it expires, for browsers that don't keep the cookie, such as over plain HTTP on a LAN address.
- **Retry:** if a request still gets a `401`, for example because the cookie was cleared, the check runs and the request is repeated once. A second `401` fails. `fetchClub` does the same.
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

1. Add the release at the **top** of `RELEASES` in `src/lib/releases.ts`, with its build, date, kind and changes. Each change has a kind (`new`, `improved` or `fixed`) and the keys of its title and body messages. It can also have a `link`, the key of its label and a path on this site, shown under the body to take players to the page it's about.
2. Write both messages in `messages/en.ts` and `messages/ja.ts`, under `releases.<name>.title` and `releases.<name>.body`. The tests fail if one language is missing.

Writing the notes:

- Write for players, not developers: say what changed for them and why it helps. No issue or pull request numbers, no code, no setup or hosting details.
- Use the labels players see in the app (such as **Include full clubs**), and the game's own terms in Japanese.
- Title as a short phrase without a full stop. Body as one sentence that starts with what the player can now do, such as "Track goals, compare results with your plan, and carry savings forward." Don't narrate the fix or add caveats.
- A hotfix lists only `fixed` changes. Start each title with "Fixed" and say in plain words what was wrong, such as "Searching by trainer ID no longer says your trainer wasn't found."
- Never edit the notes of a published release except to fix a mistake. Describe later changes in a new release.
