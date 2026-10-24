# uma-stable-match-server

The API server for Uma Stable Match. It scans uma.moe for candidate clubs on a schedule, keeps the results in memory, and recommends clubs for a trainer ID or a daily fan pace.

## Run

```bash
cp .env.example .env    # then add your uma.moe API key after UMAMOE_API_KEY=
cargo run -p uma-stable-match-server
```

Run these from the repository root; on Windows, use PowerShell. Building needs Rust and a C compiler; the main README's [Prerequisites](../../README.md#prerequisites) show how to install them on macOS, Windows and Linux. The server reads the first `.env` it finds in the directory it's started in or a parent directory. Real environment variables take precedence over `.env`.

To run it in Docker instead, see [Installation](../../README.md#installation) in the main README.

## Configuration

| Variable | Default | Meaning |
|---|---|---|
| `UMAMOE_API_KEY` | none (required) | uma.moe API key. Create one in your uma.moe settings |
| `UMAMOE_BASE_URL` | `https://uma.moe` | API base URL override, for testing against a mock |
| `HOST` | `127.0.0.1` | IP address to listen on. Use `0.0.0.0` in a container, or `::` for all IPv6 and IPv4 interfaces |
| `PORT` | `8080` | Port to listen on |
| `RUST_LOG` | `info` | Log filter, such as `debug` or `uma_stable_match_server=debug`. At `debug`, the HTTP libraries can log request URLs, which include trainer IDs |
| `RUST_LIB_BACKTRACE` | none | `1` to capture a stack trace for each failed uma.moe request. See [Logs](#logs) |
| `LOG_FORMAT` | `text` | `text` for readable lines, or `json` for one JSON object per line, for a log search service. See [Logs](#logs) |
| `SCAN_RANK_FROM` | `80` | Best monthly rank to scan |
| `SCAN_RANK_TO` | `900` | Worst monthly rank to scan |
| `SCAN_MAX_DETAILS` | `200` | Most clubs whose member data is kept loaded, best rank first. Clubs past it load when a search needs them |
| `SCAN_INTERVAL_MINUTES` | none | Minutes between scans. Unset, the server scans again 30 minutes after uma.moe's next daily update; see [Cache and schedule](#cache-and-schedule) |
| `DETAIL_CACHE_MINUTES` | none | Minutes between refreshes of a club's member data. Unset, each club refreshes after uma.moe's daily update; see [Refresher](#refresher) and [Club details](#club-details) |
| `SHORTLIST_SIZE` | `120` | Clubs nearest your pace considered for each request |
| `REQUEST_FETCH_BUDGET` | `10` | Most clubs fetched from uma.moe while a visitor waits. At 500 ms spacing, 10 take about 5 seconds. Clubs read from Redis don't count |
| `TRAINER_CACHE_MINUTES` | `10` | Minutes a trainer's looked-up pace is reused |
| `RATE_LIMIT_PER_MINUTE` | `30` | Recommendation requests each visitor can make per minute |
| `TRAINER_LOOKUPS_PER_MINUTE` | `10` | Trainer lookups each visitor can make per minute that need a call to uma.moe |
| `TRUST_FORWARDED_FOR` | `false` | `true` to identify visitors by `X-Forwarded-For`. Only turn this on behind a reverse proxy that sets the header |
| `FRONTEND_DIR` | none | Built frontend to serve, such as `frontend/dist`. Unset, the server serves only the API |
| `REDIS_URL` | none | A `redis://` or `rediss://` URL to share club details, trainer paces and scans with other instances. Unset, everything stays in this instance's memory. See [Shared cache](#shared-cache) |
| `REDIS_KEY_SECRET` | none | Required with `REDIS_URL`: at least 32 characters, the same on every instance. Keys the hash that replaces trainer IDs in Redis |
| `METRICS_ADDR` | none | IP address and port to serve Prometheus metrics and the full [status](#status) on, such as `127.0.0.1:9464`. Unset, no metrics are recorded. See [Metrics](#metrics) |

Blank values count as unset. These stop the server at startup, with an error message and exit code 1:

- a missing key, or a `HOST` that isn't an IP address (hostnames such as `localhost` aren't accepted)
- a `PORT` outside 1 to 65535, a `TRUST_FORWARDED_FOR` other than `true` or `false`, or a `LOG_FORMAT` other than `text` or `json`
- a scan, cache or rate limit setting that isn't a positive whole number, or `SCAN_RANK_FROM` above `SCAN_RANK_TO`
- a `FRONTEND_DIR` without an `index.html`
- a `REDIS_URL` without a `redis://` or `rediss://` scheme, a missing or short `REDIS_KEY_SECRET`, or a Redis that can't be reached within 5 seconds
- a `METRICS_ADDR` that isn't an IP address and port, or that can't be listened on

The API key, `REDIS_URL` (which can hold a password) and `REDIS_KEY_SECRET` are never logged, never included in errors and are redacted in `Debug` output. `.env` is git-ignored; never commit it.

## Scanner

`scan` builds a new [club pool](#club-pool) and decides which clubs' details to keep loaded:

1. List clubs by monthly rank, best first, 100 per page and up to `SCAN_RANK_TO`. Stop after a page whose last club is ranked `SCAN_RANK_TO` or worse, or after a short or empty page. As a safety stop, never list more than enough pages to reach `SCAN_RANK_TO`, plus one (10 pages with the defaults), so any window can be listed in full.
2. Decide which month to use. Early in a game month the previous month is used instead; see [Early in the month](#early-in-the-month).
3. Build a `ClubPool` from the clubs ranked `SCAN_RANK_FROM` to `SCAN_RANK_TO`. `ListedClub::from_circle` drops archived clubs, clubs with fewer than 8 members, and rows missing points, a member count or `last_updated`. Closed and full clubs are kept, so each request can include or exclude them.
4. Copy in the exact medians of clubs that are still cached for that month, then swap the new pool in.
5. Take up to `SCAN_MAX_DETAILS` joinable clubs from the pool, best rank first. If a club's details come back without club information, the listed details are used.
   - **With a [refresher](#refresher)**, as the server always has: put them on its schedule, put up to `SCAN_MAX_DETAILS` full clubs on its [full-club schedule](#full-clubs), and finish. The refresher loads them.
   - **Without one**: prefetch their member data in the background lane, so visitors' requests go first. Full clubs are only loaded when a search includes them.

While prefetching, a club whose details fail to load is logged and skipped. A failed list request fails the whole scan and leaves the previous pool in place. `Scan` returns the number of candidates (the pool size, full clubs included), how many clubs were prefetched (with a refresher, how many were already loaded), how many failed, the game month of the listed data, and when uma.moe last updated it (`updated_at`, the latest `last_updated` listed).

With the client's default 500 ms rate limit, a default scan with a prefetch (without a refresher) takes about 80 seconds: around 9 list pages plus one lookup per candidate, about 150 at the time of writing. Clubs still cached from the previous scan aren't fetched again. Without the prefetch, a scan takes a few seconds.

## Club pool

The club pool is the first stage of two-stage matching (#87). It picks the clubs worth scoring for one trainer, using only the list endpoint's data. Each scan builds a new pool, and [recommendations](#choosing-clubs) use its shortlist to decide which clubs to load.

### Game months

uma.moe's game month starts on the **2nd at 00:00 JST** (UTC+09:00), so the 1st still belongs to the previous month. `game_month_start` and `elapsed_days` follow this. A list row's `last_updated` is the daily 00:00 JST snapshot, so `elapsed_days` at that time is a whole number of days.

### Early in the month

On the first two days of a game month (the 2nd and 3rd in JST), there are fewer than 3 days of data, which isn't enough for a pace. Until the 4th, the whole flow uses the previous game month instead:

| Part | Normal | Early in the month |
|---|---|---|
| Pool estimate | `monthly_point ÷ elapsed days ÷ member_count` | `last_month_point ÷ days in the previous game month ÷ member_count` (`from_circle_last_month`) |
| Club details | Current month | `CircleRequest::for_month` with the previous month |
| Trainer pace | This month's `daily_fans`, from the club in this month's rankings | Last month's `daily_fans`, from the club in last month's rankings, or the 7-day gain ÷ 7 from `rankings_gains` if that month has too little data |
| `month` in responses | Current game month | Previous game month |

The scan decides the month from the list's snapshot time (`is_early_month`), so clubs and trainers always use the same month. The detail cache and the trainer pace cache keep each month's data separately, so the switch on the 4th never mixes months. A detail load that started under the old month doesn't write its median into the new pool.

### Estimate

`ListedClub::from_circle` estimates each club's pace from its list row:

```text
raw rate = monthly_point ÷ elapsed days ÷ member_count
```

The raw rate estimates the club's **mean** daily gain per member. Clubs are left out when they're archived, have fewer than 8 members, or are missing points, a member count or a valid `last_updated`, and before a full day of the month has passed. Closed and full clubs stay in the pool, so each request can include or exclude them.

### Calibration

Fit uses the **median** of current members, which is usually below the mean. `Calibration` corrects for this with one factor for the whole pool:

```text
c = median of (exact median ÷ raw rate)   over clubs with loaded details
estimated median = exact median if known, otherwise c × raw rate
```

`club_median` takes the exact median from the scoring crate's `score_club`, so it is exactly the median the fit formula uses. `ClubPool::set_exact_median` records one for a club, recalculates `c` from every exact median, and re-sorts the pool. `set_exact_medians` does the same for several clubs at once. With fewer than 20 valid samples, `c` is 1.0.

### Shortlist

`ClubPool::shortlist_matching(target, k, filter)` returns the `k` clubs whose estimated median is nearest `target`, which is normally `TARGET_MEDIAN_RATIO` (0.85) × your pace. Distance is measured in log space, so half and double the target are equally far:

```text
distance = |ln(estimated median ÷ target)|
```

The pool is sorted once by estimate. A query binary-searches to the target and walks outwards in both directions, skipping the clubs `filter` leaves out: closed clubs unless `include_closed`, and full clubs unless `include_full`. `ClubPool::shortlist(target, k, include_closed)` is a shorthand that always leaves full clubs out. That costs O(log n + k). Ties go to the lower club ID, and a property test checks that the result always matches sorting every club by distance. Clubs are re-sorted only when exact medians are added, not for each query.

### Shortlist size

`DEFAULT_SHORTLIST_SIZE` is 120. This was measured by comparing the top 10 from scoring every club with the top 10 from scoring only the shortlist (recall@10), across 40 paces from 0.4M to 8M fans a day:

| Pool | Shortlist | Mean recall@10, fit only / closed included / with balance | Worst pace |
|---|---|---|---|
| 500 synthetic clubs | 60 (12%) | 0.88 / 0.85 / 0.96 | 0.4 |
| 500 synthetic clubs | 120 (24%) | 0.98 / 0.97 / 0.995 | 0.8 |
| 153 real clubs (ranks 80–900) | 30 (20%) | 0.72 / 0.63 / 0.75 | 0.3 |
| 153 real clubs | 90 (59%) | 1.00 / 0.89 / 1.00 | 0.8 |
| 153 real clubs | 120 (78%) | 1.00 / 0.99 / 1.00 | 0.9 |

The synthetic pools (`tests/shortlist_recall.rs`) need a mean of at least 0.95 and at least 0.8 at every pace. On real data the estimate is much rougher: real clubs sit in a narrow pace band, and their median-to-mean ratio varies widely. So a shortlist needs a large share of the pool to keep recall high. A larger shortlist means more club details to look up for each request.

Scoring twice the shortlist, as searches do, is checked separately on three synthetic pools of 3,000 clubs with an exact median for every club: the top 10 must match scoring every club with a mean recall of at least 0.99 and at least 0.9 at every pace.

Exact medians help most when many clubs have them. With an exact median for every synthetic club, a shortlist of 60 reaches 0.995. On real data, with exact medians for the 80 prefetched clubs (about half the pool), a shortlist of 30 rises from 0.72 to 0.77, and one of 60 barely changes. Fit also depends on where you'd place among the members, so on real data being close in median isn't a strong enough signal on its own.

## Club details

`ClubDetails` is a shared cache of club details (`Club` with members), built on [`moka`](https://crates.io/crates/moka). Clones share one cache.

- **Single flight.** `get(circle_id, priority)` fetches a club only if it isn't cached. Concurrent calls for the same club share one request to uma.moe. Failed fetches aren't cached, so the next call tries again.
- **Refresh.** `refresh(circle_id, priority)` fetches a club from uma.moe even if it's cached, then replaces the cached copy in memory and in the shared cache, and the pool's exact median. If the fetch fails, the cached copy stays.
- **Update time.** Each entry keeps the club's `last_updated` from uma.moe, which `last_updated(circle_id)` returns for a cached club.
- **Expiry.** Each entry is due for its next refresh at the time the [refresher](#refresher) would use: `last_updated` + 24 hours + 30 minutes, an hour from now if uma.moe hasn't updated the club yet, or `DETAIL_CACHE_MINUTES` from now when set (`DetailConfig::refresh`). With `DetailConfig::refresher` set, entries stay in memory until they're replaced; otherwise they expire when due. Either way the cache holds at most `max_clubs` (2,000 by default).
- **Exact medians.** Each successful load writes the club's exact median into the shared `ClubPool`. After a scan builds a new pool, `restore_exact_medians` copies in the medians of clubs that are still cached.
- **Background loading.** `queue(circle_id)` adds a club for `BackgroundLoader::run` to fetch in the background lane. Clubs that are already cached or queued are skipped, and once `queue_size` clubs are waiting (1,000 by default), `queue` returns `false`. A failed background load is logged, and the club leaves the queue.
- **Lanes.** Visitors' fetches use the interactive lane, so they go ahead of queued background loads. If a background load for a club is already running, a visitor asking for that club waits for it rather than starting a second fetch.
- **Long pauses.** When uma.moe answers 429 with a `Retry-After` over 30 seconds, visitors' club fetches and trainer lookups fail straight away rather than hold the page; the clubs are queued for a background load instead. Scans and the refresher wait out the whole pause.

`queued()` and `cached_clubs()` report the queue length and the approximate cache size.

## Refresher

A `Refresher` keeps club details current, so a scan only lists clubs. `ClubDetails::refresher` returns it when `DetailConfig::refresher` is set, and the server runs it in the background.

uma.moe updates club data once a day, at 15:00 UTC, and each club's `last_updated` shows when. The refresher keeps a schedule of clubs, ordered by when each is due, then by rank:

| Event | Next refresh |
|---|---|
| A scan lists a club among its `SCAN_MAX_DETAILS` | Straight away. Clubs already scheduled keep their time |
| A search loads a club in the pool | After uma.moe's next update |
| A refresh brings a new `last_updated` | `last_updated` + 24 hours + `margin` (30 minutes) |
| uma.moe hasn't updated the club yet | After `recheck` (1 hour) |
| The refresh fails | After `retry` (5 minutes). The old details are served meanwhile |
| A scan no longer lists the club | Dropped from the schedule |

- **Refreshing.** `refresh` fetches the club in the background lane and replaces its details and exact median. A club whose cached details are still current isn't fetched again, only rescheduled.
- **Speed.** Clubs ranked up to `fast_ranks` (5,000) refresh as fast as the client allows. After each club ranked lower, the refresher waits as long as the fetch took, which halves the pace and leaves room for visitors. The client's [backoff](../umamoe-client/README.md#backing-off) slows both.
- **Override.** With `DETAIL_CACHE_MINUTES` set (`RefreshConfig::every`), each club is refreshed that many minutes after its last refresh, whatever uma.moe's update time.

`scheduled()` returns how many clubs are on either schedule, and `next_refresh(circle_id)` when a club is next due.

### Full clubs

Full clubs have a schedule of their own, with the same timing, so joinable clubs are always refreshed first:

- **Order.** The refresher takes a full club only when no joinable club is due. While joinable clubs are waiting, full clubs wait too, even if they became due earlier.
- **Speed.** Every full club refreshes at half pace, whatever its rank.
- **Moving.** A scan moves a club that filled up to the full-club schedule, and one with an open spot again back, using the pool's latest member counts. A club that moves is refreshed again soon after.
- **Searches.** A full club loaded for a search joins the full-club schedule.

### With Redis

Every instance shares one schedule, and one instance at a time refreshes:

- **Schedule.** A sorted set at `usm:v1:scan:<from>-<to>-<max>:schedule`, with no expiry, and one for full clubs at `…:schedule-full`. Each score holds the due time, rounded up to the second, then the rank, so clubs due at the same time come out best rank first. A listing on any instance updates it, and clubs loaded on any instance join it.
- **Lease.** Each instance's refresher tries to take `usm:v1:scan:<from>-<to>-<max>:refresher` every 10 seconds. The holder refreshes and renews it every 20 seconds; it lasts 60 seconds, so another instance takes over within about 70 seconds after a crash or redeploy.
- **Already refreshed.** Before fetching a due club, the refresher checks the club's due time in memory and then in Redis. A club another instance has loaded since is only rescheduled.
- **In memory.** Each instance's copies expire when due, as in [Club details](#club-details), so they're read again from Redis once the holder has refreshed them.
- **Redis down.** No instance can take the lease, so nothing is refreshed in the background. Searches still load due clubs from uma.moe when they need them.

## Shared cache

`SharedCache` is a second cache layer for running several server instances against one store. On a local miss, `ClubDetails` and `PaceLookup` check it before calling uma.moe, and store what they load in it, under keys such as `usm:v1:club:<id>:<month>`. Paces expire as they do in memory. Club values hold the club, its `last_updated` and when it's due for its next refresh, and stay in Redis a day past that, so they're there until replaced. A club found there still gives the pool its exact median.

- **Due clubs.** A club past its next refresh is fetched from uma.moe again. If that fails, the copy in Redis is served and kept in memory for `RefreshConfig::retry` (5 minutes).
- **Older values.** A value stored by an older version, without a due time, counts as due.

Without `REDIS_URL`, the shared cache stores nothing (`SharedCache::none()`) and each instance works alone. With it, the server uses Redis:

| Situation | What happens |
|---|---|
| Start-up | The server connects and sends `PING`. If Redis can't be reached within 5 seconds, it stops with exit code 1, so a wrong URL is caught straight away |
| Each read or write | Times out after 500 ms. Any error or timeout counts as a miss, and the instance carries on with its own cache and uma.moe |
| Redis goes away | One warning is logged; the connection reconnects by itself, and one message is logged when Redis is back. With [metrics](#metrics) on, `redis_up` shows the state and `redis_errors_total` counts each failure |

**Looked-up trainer IDs never reach Redis.** Pace keys use the first 32 hex characters of HMAC-SHA256(`REDIS_KEY_SECRET`, trainer ID), so every instance needs the same secret. Club keys keep the club ID, which isn't personal, and club data is stored as uma.moe publishes it. Run Redis with persistence (for example `--appendonly yes`), so club details, the published scan and the refresh schedule survive a Redis restart instead of being loaded from uma.moe again. The `redis` profile in `docker-compose.yml` runs it this way, with its data in a volume; see [Docker details](../../README.md#docker-details). On Fly.io, [`deploy/redis`](../../deploy/redis) does the same with a password.

Still per instance: rate limits and the background queue. The refresh schedule is shared; see [Refresher](#with-redis).

### Shared scans

With Redis, instances that have the same `SCAN_RANK_FROM`, `SCAN_RANK_TO` and `SCAN_MAX_DETAILS` share one scan, so uma.moe is listed once per scan however many instances run. Whenever a scan is due, one Redis script decides what the instance does:

| Situation | What the instance does |
|---|---|
| A published scan is still current | Loads it unless it already has, then waits until the next scan is due |
| No recent scan, and the scan lock is free | Takes the lock, scans, and publishes the result. If a scan in progress was saved, it resumes that scan instead of listing again |
| Another instance holds the lock | Builds its pool from that scan's saved club list once there is one, so it can answer searches, then reports `scanning: true` and checks again every second |
| Redis doesn't answer | Scans on its own |

- **Lock.** It lasts 60 seconds and is renewed every 20 seconds while scanning. Shutdown releases it; if an instance crashes, another takes over within 60 seconds.
- **Publishing.** After listing and scheduling the clubs, the leader publishes them and marks them current until the next scan is due: 30 minutes after uma.moe's next update, or for one `SCAN_INTERVAL_MINUTES` when set. It keeps them for twice that. The others build their pool from those rows and load club details from Redis, so they barely call uma.moe. An instance that starts later loads the published scan straight away.
- **Resuming.** As soon as the leader has listed the clubs, it saves the list in Redis with when it was listed. An instance that takes the lock later, after a restart, redeploy or crash, continues that scan instead of listing again, if the list is less than `SCAN_INTERVAL_MINUTES` old (180 minutes when unset) and from the current game month (counting the [early-month](#early-in-the-month) switch). The resumed scan keeps its original start time in [`/status`](#status) and the metrics. Publishing the finished scan deletes the saved list.
- **Failures.** A failed scan is published too: every instance records the same error, keeps its last results, and waits until the next scan is due.
- **Missing data.** If a scan is marked current but its rows are gone from Redis, the instance scans on its own.

The keys are `usm:v1:scan:<from>-<to>-<max>:lock`, `:fresh`, `:latest`, `:progress`, `:progress-id`, `:schedule`, `:schedule-full` and `:refresher`, so instances with different scan windows don't share.

### Redis tests

The Redis tests in `tests/redis.rs` need a Redis server. Without `REDIS_TEST_URL` they're skipped; CI runs them against a Redis service container. To run them locally, start a Redis that keeps nothing on disk:

| Platform | Install and start Redis |
|---|---|
| macOS | `brew install redis`, then `redis-server --save "" --appendonly no` in its own terminal |
| Linux | `sudo apt install redis-server` (Debian, Ubuntu) or `sudo dnf install redis` then `sudo systemctl start redis` (Fedora). The package runs Redis as a service on port 6379 |
| Windows | Redis has no Windows build. Use Docker, below, or install it in [WSL](https://learn.microsoft.com/windows/wsl/install) as on Linux |
| Any, with Docker | `docker run -d --rm --name redis-test -p 127.0.0.1:6379:6379 redis:7.4-alpine --save "" --appendonly no`, and `docker stop redis-test` afterwards |

Then run the tests with the URL set:

```bash
REDIS_TEST_URL=redis://127.0.0.1:6379 cargo test
```

In PowerShell on Windows:

```powershell
$env:REDIS_TEST_URL="redis://127.0.0.1:6379"; cargo test
```

## Cache and schedule

Scan results, the club pool and club details live in memory, and with Redis also in Redis. When the server starts it scans straight away. It scans again 30 minutes after uma.moe's next daily update: 24 hours after the scan's `updated_at`, plus the refresher's `margin`. If uma.moe hasn't updated yet, or the scan failed, it tries again an hour later. With `SCAN_INTERVAL_MINUTES` set, it scans again that long after each scan finishes instead. Until the first scan has listed the clubs there are no results; with the defaults that takes a few seconds. From then on, searches score the clubs loaded so far, fetch up to `REQUEST_FETCH_BUDGET` more, and report the rest as still loading, while the [refresher](#refresher) finishes loading details (about 80 seconds with the defaults). With Redis, a server that starts while a published scan is current loads it in a second or two instead, and follows that scan's schedule. Its memory starts empty, but searches read shortlisted clubs from Redis without using `REQUEST_FETCH_BUDGET`, so clubs already in Redis are scored from the first search. A server that starts during a scan resumes it from the saved club list. See [Shared scans](#shared-scans).

- **No overlapping scans.** A new scan won't start while one is running.
- **One scan for all instances.** With Redis, one instance scans and the others load its result; see [Shared scans](#shared-scans).
- **Failures keep the last results.** A failed scan is logged and recorded as the last error, and the previous results stay in place. The next successful scan replaces the results and clears the error.
- **Shutdown cancels a running scan.** The cache then keeps whatever it had before that scan started.

## Serving the frontend

With `FRONTEND_DIR` set, the server also serves the built web page, so one process runs the whole site:

```bash
(cd frontend && npm run build)
FRONTEND_DIR=frontend/dist cargo run -p uma-stable-match-server
```

| Request | Response |
|---|---|
| A file in the directory | The file |
| `/assets/*` | Cached for a year as `immutable`. Vite puts a content hash in these file names, so a new build gets new names |
| Any other file, including `index.html` | `Cache-Control: no-cache`, so browsers check for a new version on each visit |
| A path without a file extension, such as `/clubs` | `index.html`, so the app handles the route |
| A missing path with a file extension | `404` |
| `/api` or anything under it | Always the API. Unknown API paths return `404 {"error":"not found"}`, never the page |

### Security headers

Every response, from the API and the frontend alike, carries these headers:

| Header | Value |
|---|---|
| `Content-Security-Policy` | `CONTENT_SECURITY`: scripts, styles, fonts and requests only from the site itself, images also from `data:` URLs, no plugins, and no framing |
| `X-Content-Type-Options` | `nosniff` |
| `Referrer-Policy` | `no-referrer` |
| `X-Frame-Options` | `DENY` |
| `Strict-Transport-Security` | `max-age=31536000`: browsers use HTTPS for the next year. They ignore it on plain-HTTP responses, so local setups are unaffected |

The policy allows no inline scripts or styles, so the frontend must not add any. That's why the theme script lives in its own file.

### Request timeout

A request still running after 30 seconds is cut off with `504 {"error":"the request took too long; try again"}`, so slow or held-open requests can't pile up. 30 seconds is the longest `Retry-After` pause a visitor's request waits out; the web page gives up after 15 seconds on its own.

## Endpoints

All endpoints are under `/api/v1`.

| Method | Path | Response |
|---|---|---|
| `GET` | `/api/v1/health` | `200 {"status":"ok"}` |
| `GET` | `/api/v1/recommendations` | Ranked clubs for a trainer or a pace, below |

There's no public status endpoint. The page takes the data's age from each search's `updated_at`, and the full scan state is served privately at [`/status`](#status), next to the metrics.

### Recommendations

`GET /api/v1/recommendations` scores the cached clubs nearest your pace with [`uma-stable-match-scoring`](../scoring/README.md) and returns them best first.

| Parameter | Required | Default | Meaning |
|---|---|---|---|
| `trainer_id` | One of these two | | Your trainer ID. The server works out your pace from your own fan data this month |
| `pace` | One of these two | | Your daily fan gain, such as `7600000` |
| `balance` | No | `false` | `true` turns on balance scoring at weight 0.5 |
| `balance_weight` | No | | Balance strength, from 0 to 1. Setting it turns balance scoring on |
| `include_closed` | No | `false` | `true` to include clubs that aren't accepting members |
| `include_full` | No | `false` | `true` to include clubs with no open spot |
| `limit` | No | `50` | Most clubs to return |

`balance=false` together with `balance_weight` is rejected, because the request contradicts itself.

#### Choosing clubs

1. Check the pace and balance weight. An invalid request fails before anything is fetched.
2. Shortlist the `SHORTLIST_SIZE` clubs whose estimated median is nearest `0.85 ×` your pace. Closed clubs are left out unless `include_closed=true`, and full clubs unless `include_full=true`.
3. Take the clubs nearest your pace, twice `SHORTLIST_SIZE` of them, the shortlist first. Read their member data from memory, then the rest from Redis in one `MGET`, keeping those not yet due for a refresh in memory. Then fetch up to `REQUEST_FETCH_BUDGET` of the shortlisted clubs still missing from uma.moe in the interactive lane, nearest first. Requests that need the same club share one fetch. The clubs past the shortlist are only scored if they're already stored.
4. Queue the other uncached shortlisted clubs, and any that failed, for background loading.
5. Score the loaded clubs. Without `include_full=true`, full clubs are dropped first, judged by the pool's latest list data rather than their cached details, which can be a day old.

Only these clubs are copied and scored, so a search costs the same however large the pool is. Clubs further from your pace can't fit well, so this gives the same results as scoring every club: on three synthetic pools of 3,000 clubs with exact medians, the normal case once the refresher has loaded them, the top 10 matches scoring every club (mean recall 1.0, or 0.9975 with closed clubs). `coverage` says how complete a response is.

#### Looking up a trainer

With `trainer_id`, the server finds the trainer's club and their snapshot in it (the latest month if there's more than one), then calculates their pace with `daily_pace`, which needs at least 3 days of data and a gain above zero:

1. **Their club, from monthly rankings.** `rankings_monthly` gives the club the trainer is in for the month, matching the exact trainer ID. uma.moe's own trainer lookup returns a trainer's oldest club once they've changed clubs, so it isn't used first.
2. **Their fans, from that club.** The club comes through the detail cache (memory, then Redis, then uma.moe), so a club already loaded costs no extra call.
3. **Fallback.** With no rankings row, no club, a club that doesn't list them, or a failed rankings or club request, the server uses uma.moe's trainer lookup (`CircleRequest::by_viewer_id`) instead.

[Early in the month](#early-in-the-month), it uses last month's rankings and data, falling back to the trainer's 7-day gain.

- **Cached for `TRAINER_CACHE_MINUTES`** (10 by default). A found pace, and a "not found" or "not enough data" answer, is reused for that long, so repeat requests don't call uma.moe again. Failed uma.moe requests aren't cached.
- **IDs stay out of logs.** Trainer IDs never appear in log messages or error responses, only in the response to the trainer's own request. Failed lookups log only the HTTP status.
- **No lookup without results.** Before the first scan has listed the clubs, the endpoint returns `503` without contacting uma.moe.

#### The trainer's own club

The lookup also notes the club it found the trainer in. That club is left out of `clubs` and returned as `your_club`, scored as if the trainer were joining it:

- **Without their own row.** The trainer's snapshot is removed and the club counts as one member smaller, so `your_place` and `club_size` match a newcomer at their pace. `member_count` and `open_spots` still show the real club.
- **Closed and full clubs count,** since the trainer is already in it. Its details come through the detail cache like any other club's.
- **`result_rank`** is where it would sit among the scored clubs, from 1. Above `limit`, it's below the returned clubs.

`your_club` is `null` for `pace` requests, when no club was found, when the club fails to load, or when it can't be scored, for example with fewer than 8 rankable members. A failed load is logged but never fails the search.

#### Response

Clubs with fewer than 8 rankable members aren't returned, and with balance scoring on, neither are clubs whose balance can't be measured.

```json
{
  "trainer": { "trainer_id": 123456789012, "pace_per_day": 7600000.0 },
  "options": { "balance_weight": 0.5, "include_closed": false, "include_full": false, "limit": 50 },
  "updated_at": "2026-03-18T09:00:00Z",
  "month": "2026-03",
  "coverage": { "considered": 4980, "shortlisted": 120, "scored": 240, "pending": 0 },
  "total": 236,
  "clubs": [
    {
      "circle_id": 1,
      "name": "Example Club",
      "url": "https://uma.moe/circles/1",
      "tier": "A+",
      "join_style": "open",
      "policy": 2,
      "member_count": 25,
      "open_spots": 5,
      "monthly_rank": 120,
      "last_month_rank": 150,
      "comment": "Made-up description",
      "your_place": 9,
      "club_size": 26,
      "ranked_members": 24,
      "median_per_day": 6400000.0,
      "fit": 0.0562,
      "score": 0.1062,
      "balance": {
        "score": 90,
        "gini": 0.12,
        "passengers": 1,
        "counted": 24,
        "bottom_quarter_per_day": 3100000.0,
        "top3_share": 0.21
      }
    }
  ],
  "your_club": {
    "circle_id": 2,
    "name": "Home Club",
    "url": "https://uma.moe/circles/2",
    "tier": "S",
    "join_style": "closed",
    "policy": 2,
    "member_count": 30,
    "open_spots": 0,
    "monthly_rank": 40,
    "last_month_rank": 38,
    "comment": "Made-up description",
    "your_place": 24,
    "club_size": 30,
    "ranked_members": 29,
    "median_per_day": 11200000.0,
    "fit": 0.4321,
    "score": 0.4821,
    "balance": null,
    "result_rank": 37
  }
}
```

| Field | Meaning |
|---|---|
| `trainer.trainer_id` | The trainer ID you asked about, or `null` when you gave a `pace` |
| `trainer.pace_per_day` | The pace used: yours from uma.moe, or the `pace` you gave |
| `options` | The settings used, after defaults: `balance_weight` is `null` when balance scoring is off |
| `updated_at` | When the clubs were scanned. Before the first scan finishes, when its club list was fetched |
| `month` | Game month of the club data, `YYYY-MM` |
| `coverage.considered` | Clubs in the pool that your options allow. Each was compared with your pace by its estimate |
| `coverage.shortlisted` | Clubs on the shortlist for your pace |
| `coverage.scored` | Clubs with member data that were scored, at most twice the shortlist |
| `coverage.pending` | Shortlisted clubs still loading in the background. Above 0, searching again shortly can give more results |
| `total` | How many clubs were scored, before `limit` |
| `clubs` | Up to `limit` clubs, best first. The trainer's own club is never one of them |
| `your_club` | With `trainer_id`, the trainer's [own club](#the-trainers-own-club) with the same fields as `clubs`, plus `result_rank`. Otherwise `null` |

- `join_style` is `open`, `approval`, `closed` or `null`. It's `null` when the style is unknown, including any new style the server doesn't recognise yet.
- `tier` runs from `D` to `SS`, or is `null` when unknown.
- `policy` is uma.moe's playstyle code.
- `balance` is `null` unless balance scoring is on.
- Lower `fit` and `score` are better.

The [scoring README](../scoring/README.md) explains every value.

#### Errors

Errors come back as `{"error": "..."}`:

| Status | When |
|---|---|
| `400` | Any of these:<br>• the query string can't be read, for example because a parameter is repeated<br>• neither or both of `trainer_id` and `pace` are given<br>• `trainer_id` isn't a trainer ID<br>• `pace` isn't a number above zero<br>• `balance_weight` isn't a number from 0 to 1<br>• `balance=false` is given with `balance_weight`<br>• `balance`, `include_closed` or `include_full` isn't `true` or `false`<br>• `limit` isn't a positive whole number |
| `404` | uma.moe doesn't know the trainer, or they aren't in their club's data |
| `422` | The trainer has too little fan data this month to work out a pace |
| `502` | uma.moe couldn't be reached for the trainer lookup |
| `429` | Too many requests from this visitor. The `Retry-After` header says how many seconds to wait |
| `503` | The first scan hasn't listed the clubs yet. Try again shortly |
| `504` | The request took longer than 30 seconds. See [Request timeout](#request-timeout) |

### Rate limits

`/api/v1/recommendations` is limited per visitor. Health isn't limited.

- **All requests:** `RATE_LIMIT_PER_MINUTE` (30 by default).
- **Trainer lookups:** `TRAINER_LOOKUPS_PER_MINUTE` (10 by default). This only counts `trainer_id` requests that need a call to uma.moe. Cached trainers and `pace` requests don't use it, so a public instance can't be used to flood uma.moe.

Each limit allows a burst of its full size, then frees one request every 60 ÷ limit seconds. With the defaults, that's one request every 2 seconds and one trainer lookup every 6 seconds. Over the limit, the response is `429` with `Retry-After` and an error such as `"too many requests; try again in 2 seconds"`.

Visitors are identified by IP address:

- **IPv6 addresses** are grouped by their first 64 bits, the usual size of one home network.
- **IPv4-mapped IPv6 addresses** count as IPv4.
- **Behind a reverse proxy**, every request comes from the proxy's address. Set `TRUST_FORWARDED_FOR=true` and the server uses the right-most `X-Forwarded-For` address, the one your proxy added. Anything a client writes further left is ignored. Don't turn it on without a proxy, or clients can pick their own address.

IP addresses are kept in memory only while they're being limited, and they're never logged.

## Logs

Logs go to stdout. With `LOG_FORMAT=json`, each line is a JSON object with the timestamp, level, message, module (`target`) and the source file and line that logged it (`filename`, `line_number`), plus the event's own fields, such as `circle_id` and `error`:

```json
{"timestamp":"2026-10-01T09:00:00.000000Z","level":"WARN","message":"club refresh failed; retrying later","circle_id":900000101,"origin":"network","error":"request to uma.moe timed out","target":"uma_stable_match_server::refresher","filename":"backend/server/src/refresher.rs","line_number":348}
```

### Searches

Each search logs one `search` line, so it can be found and rerun. It holds the `query` string, `status`, the `trainer_id` for a trainer search, `pace_per_day`, `month`, the `coverage` counts (`considered`, `shortlisted`, `scored`, `pending`), `total`, the top ten `top_clubs`, and `duration_ms`. It also says how deep into the rankings the search went:

| Field | Meaning |
|---|---|
| `shortlist_rank` | The worst monthly rank among the shortlisted clubs |
| `pick_rank` | The worst monthly rank among the top ten picks with an open spot |
| `pool_bottom` | Whether the shortlist included the slowest club in the pool that the search's options allow. Searches that do want slower clubs than the scan window has |

A search that fails logs `search failed` with the `query`, `status`, `error` and `duration_ms`: a warning for a server error, otherwise at info level.

### Errors and stack traces

Error lines carry an `origin`: `umamoe` for an error status, rate limit or unexpected response from uma.moe, `network` when no response came back, and `server` for a problem on our side.

- **Failed uma.moe requests.** Each call that fails, after any retries, logs one `uma.moe request failed` warning with the `endpoint`, `lane`, `origin`, HTTP `status`, uma.moe's own message (`umamoe_error`), the number of `attempts`, the `error` and its `cause`, and a `backtrace` showing which code made the call. A 404 isn't logged: it means the club or trainer doesn't exist. The backtrace is only captured with `RUST_LIB_BACKTRACE=1`, which `fly.toml` sets; without it the field reads `disabled backtrace`.
- **Panics.** A panic logs one `panicked` error with the `panic` message, its `location` and a full `backtrace`, and the request gets `500 {"error":"internal error"}`.
- **Line numbers.** Release builds keep line tables (`debug = "line-tables-only"` in the workspace `Cargo.toml`), so backtraces show files and lines.

A log search service such as Axiom can then filter by those fields. If a log shipper wraps each line in its own record, as Fly.io's does, the line arrives as a string in `message`; read its fields with `parse_json(message)` in Axiom.

At `info`, logs hold no trainer IDs or IP addresses. Keep `RUST_LOG` at `info` when logs leave the server.

## Metrics

With `METRICS_ADDR` set, the server records metrics and serves them in the Prometheus text format at `GET /metrics`, on that address only. The app's port never serves them. Unset, nothing is recorded.

Keep the address private: `127.0.0.1:<port>` on its own. With Docker Compose, set `METRICS_PORT` in `.env` instead: the container listens on `0.0.0.0:9464` and Compose publishes it to `127.0.0.1:<METRICS_PORT>` only. On Fly.io, `fly.toml` uses `[::]:9464`, which only Fly's private network, its metrics scraper and `fly proxy` can reach. Labels never include trainer IDs or IP addresses. Routes are labelled by their pattern, such as `/api/v1/recommendations`; paths that match no route are labelled `/api (unknown)` or `frontend`.

| Metric | Type | Labels | Meaning |
|---|---|---|---|
| `http_requests_total` | counter | `route`, `method`, `status` | Requests answered, by HTTP status code |
| `http_request_duration_seconds` | histogram | `route`, `method` | Time to answer a request |
| `search_shortlist_worst_rank` | histogram | | Per search, the worst monthly rank among the shortlisted clubs. Buckets from 1,000 to 65,000 |
| `search_pick_worst_rank` | histogram | | Per search, the worst monthly rank among the top ten picks with an open spot |
| `search_pool_bottom_total` | counter | | Searches whose shortlist reached the slowest club in the pool. Divided by `search_shortlist_worst_rank_count`, it's the share of searches the scan window was too short for |
| `rate_limited_total` | counter | `limit`: `requests` or `trainer_lookups` | Requests turned away by a [rate limit](#rate-limits) |
| `scan_phase` | gauge | | `0` idle or waiting for another instance's scan, `1` listing clubs, `2` loading club details |
| `scan_listed_clubs` | gauge | | Clubs in the rank window listed so far in this scan |
| `scan_clubs_to_load`, `scan_clubs_loaded`, `scan_clubs_failed` | gauge | | Progress of a scan's prefetch. The server's scans don't prefetch, as the refresher loads details |
| `scans_total` | counter | `source`: `alone`, `lead` or `published`; `result`: `ok` or `error` | Scans finished, whether run here or loaded from another instance |
| `scan_duration_seconds` | histogram | `source` | Time from a scan's start to its end. A resumed scan counts from its original start, before the restart |
| `scan_turns_total` | counter | `turn`: `alone`, `lead`, `wait` or `fresh` | What the instance did when a scan was due; see [Shared scans](#shared-scans) |
| `scan_last_success_timestamp_seconds` | gauge | | Unix time of the last successful scan |
| `pool_clubs`, `pool_exact_medians` | gauge | | Clubs in the pool, and how many have an exact median |
| `pool_calibration_factor`, `pool_calibration_samples` | gauge | | The mean-to-median factor and how many clubs it's based on |
| `umamoe_requests_total` | counter | `endpoint`, `lane`, `status` | uma.moe calls by HTTP status code, or `timeout`, `connect_error` or `transport_error`. Each retry counts as another call |
| `umamoe_request_duration_seconds` | histogram | `endpoint`, `lane` | uma.moe response time |
| `umamoe_queue_wait_seconds` | histogram | `lane` | Time waiting for the client's 500 ms spacing |
| `umamoe_allowed_rate` | gauge | | uma.moe calls per second currently allowed: 2 normally, lower while backing off after 429s |
| `club_detail_lookups_total` | counter | `source`: `memory`, `redis`, `umamoe`, `error` or `stale` | Where each club's details came from; `error` means the uma.moe fetch failed, and `stale` that a due copy from Redis was served instead |
| `pace_lookups_total` | counter | `source`: `memory`, `redis`, `umamoe` or `error` | Where each trainer's pace came from |
| `detail_cache_clubs` | gauge | | Clubs in this instance's detail cache |
| `detail_queue_clubs` | gauge | | Clubs waiting for a background load |
| `refresh_scheduled_clubs` | gauge | | Joinable clubs on the refresh schedule |
| `refresh_loaded_clubs` | gauge | | Scheduled joinable clubs whose details are stored: in Redis with Redis, otherwise in memory. Divided by `refresh_scheduled_clubs`, it's how complete the stored data is |
| `refresh_due_clubs` | gauge | | Scheduled joinable clubs due for a refresh now |
| `refresh_overdue_seconds` | gauge | | How long the longest-waiting due joinable club has waited, `0` when none are due |
| `refresh_full_scheduled_clubs`, `refresh_full_loaded_clubs`, `refresh_full_due_clubs` | gauge | | The same counts for the [full-club schedule](#full-clubs) |
| `redis_up` | gauge | | `1` while Redis answers, `0` after a failure. Absent without Redis |
| `redis_errors_total` | counter | `reason` | Failed or timed-out Redis operations, by the kind of failure |
| `redis_op_duration_seconds` | histogram | `op`: `get`, `mget`, `set`, `script`, `schedule` or `info` | Time per Redis operation, failures included |
| `redis_keys`, `redis_keys_expiring` | gauge | | Every key in Redis, and how many have a TTL |
| `redis_average_ttl_seconds` | gauge | | Redis's average TTL across the keys that have one |
| `redis_memory_used_bytes`, `redis_memory_max_bytes` | gauge | | Memory Redis uses, and its `maxmemory` limit (`0` for none) |
| `redis_expired_keys_total`, `redis_evicted_keys_total` | counter | | Keys that reached their TTL, and keys removed because Redis hit its memory limit |
| `redis_club_ttl_shortest_seconds`, `redis_club_ttl_average_seconds` | gauge | | Time to live of the scheduled clubs' details in Redis: the soonest to expire and the average. Details stay a day past their next refresh, so the soonest stays above a day while the refresher keeps up |
| `log_events_total` | counter | `level`: `warn` or `error`; `file`, `line` | Warnings and errors logged, by where in the code they were logged. Other libraries' lines have an empty `file` and `line` |

`scan_last_success_timestamp_seconds` and the `pool_*`, `detail_*` and `refresh_*` values are read from the server each time `/metrics` is requested. With Redis, the `refresh_*` and `redis_club_ttl_*` values come from one script over the shared schedule, so every instance reports the same, and the other `redis_*` gauges come from one `INFO` call; the detail cache settles its pending work first, so `detail_cache_clubs` is exact. Histograms use buckets from 1 ms to 1 hour. The `umamoe_*` metrics come from the uma.moe client; its [README](../umamoe-client/README.md#metrics) describes them in full.

To watch these on a dashboard, [observability/README.md](../../observability/README.md) sets up Prometheus and Grafana locally, with a ready-made dashboard.

## Status

With `METRICS_ADDR` set, the metrics address also serves `GET /status`: the scan, the club pool, the caches and the refresh schedule, as JSON. Like `/metrics`, it's never on the app's port. Each request counts the refresh schedule in Redis, so keep the address private. On Fly.io, reach it through the private network:

```bash
fly proxy 9464 -a uma-stable-match
```

Then open <http://localhost:9464/status>.

`GET /status` looks like this:

```json
{
  "scanning": false,
  "scan": null,
  "last_scan": {
    "finished_at": "2026-03-18T09:00:00Z",
    "clubs": 152,
    "candidates": 152,
    "failed": 0,
    "month": "2026-03"
  },
  "last_error": null,
  "pool": { "clubs": 152, "exact_medians": 152, "calibration": { "factor": 0.86, "samples": 152 } },
  "details": { "cached": 152, "queued": 0 },
  "refresh": {
    "scheduled": 152,
    "loaded": 152,
    "due": 0,
    "oldest_due_at": null,
    "full": { "scheduled": 140, "loaded": 96, "due": 44 }
  }
}
```

| Field | Meaning |
|---|---|
| `scanning` | Whether a scan is running. With Redis, also while waiting for another instance's scan |
| `scan` | The running scan's progress, or `null` when no scan is running. See below |
| `last_scan.clubs` | Clubs whose member data was already loaded when the scan finished listing |
| `last_scan.candidates` | Clubs in the pool after the scan's filters, full clubs included |
| `last_scan.month` | Game month of the listed data, `YYYY-MM` |
| `pool.clubs` | Clubs in the current pool |
| `pool.exact_medians` | Pool clubs whose exact median is known from loaded details |
| `pool.calibration` | The mean-to-median factor and how many clubs it's based on |
| `details.cached` | Clubs in the detail cache (approximate) |
| `details.queued` | Clubs waiting for a background load |
| `refresh.scheduled` | Joinable clubs on the [refresh](#refresher) schedule |
| `refresh.loaded` | Scheduled joinable clubs whose details are stored: in Redis with Redis, otherwise in this instance's memory |
| `refresh.due` | Scheduled joinable clubs due for a refresh now. It jumps after uma.moe's daily update, then falls as the refresher catches up |
| `refresh.oldest_due_at` | When the longest-waiting due joinable club became due, or `null` when none are due |
| `refresh.full` | `scheduled`, `loaded` and `due` for the [full-club schedule](#full-clubs). Its `due` only falls once the joinable clubs are done |

While a scan runs, `scan` looks like this:

```json
"scan": { "phase": "loading", "started_at": "2026-03-18T08:58:00Z", "listed": 152, "to_load": 152, "loaded": 40, "failed": 0 }
```

| Field | Meaning |
|---|---|
| `scan.phase` | `listing` clubs, `loading` club details, or `waiting` for another instance's scan (with Redis) |
| `scan.started_at` | When the scan started. A [resumed scan](#shared-scans) keeps its original start; a waiting instance shows the leader's once it has the saved club list, and `null` before that |
| `scan.listed` | Clubs in the rank window listed so far |
| `scan.to_load`, `scan.loaded`, `scan.failed` | Clubs whose details the scan loads, has loaded, and failed to load. A resumed scan counts again from 0, quickly passing clubs already in Redis |

`last_scan` is `null` until the first scan succeeds. `last_error` holds `{ "at", "message" }` for a failure since the last success. Times are RFC 3339 in UTC.

`message` is short. It never includes request URLs or text from uma.moe's error responses. The full error goes to the server log.

| Message | Cause |
|---|---|
| `listing clubs failed: uma.moe returned 503 Service Unavailable` | uma.moe answered with an error status (the status varies) |
| `listing clubs failed: uma.moe asked to slow down` | uma.moe asked for a pause longer than the caller waits out |
| `listing clubs failed: could not reach uma.moe` | Timeout or connection error |
| `listing clubs failed: unexpected response from uma.moe` | The response wasn't the expected JSON |
| `listing clubs failed: internal error` | A problem in the server itself |

## Shutdown

Ctrl+C or SIGTERM (as sent by `docker stop`) stops accepting new connections, including on the metrics address, and cancels any running scan. In-flight requests finish, and then the server exits.
