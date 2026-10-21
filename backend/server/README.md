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
| `SCAN_RANK_FROM` | `80` | Best monthly rank to scan |
| `SCAN_RANK_TO` | `900` | Worst monthly rank to scan |
| `SCAN_MAX_DETAILS` | `200` | Most clubs whose member data is prefetched in each scan |
| `SCAN_INTERVAL_MINUTES` | `180` | Minutes between scans. The default may change once uma.moe agrees how often scans can run |
| `DETAIL_CACHE_MINUTES` | `SCAN_INTERVAL_MINUTES` | Minutes a club's member data is reused |
| `SHORTLIST_SIZE` | `120` | Clubs nearest your pace considered for each request |
| `REQUEST_FETCH_BUDGET` | `10` | Most uncached clubs fetched while a visitor waits. At 500 ms spacing, 10 take about 5 seconds |
| `TRAINER_CACHE_MINUTES` | `10` | Minutes a trainer's looked-up pace is reused |
| `RATE_LIMIT_PER_MINUTE` | `30` | Recommendation requests each visitor can make per minute |
| `TRAINER_LOOKUPS_PER_MINUTE` | `10` | Trainer lookups each visitor can make per minute that need a call to uma.moe |
| `TRUST_FORWARDED_FOR` | `false` | `true` to identify visitors by `X-Forwarded-For`. Only turn this on behind a reverse proxy that sets the header |
| `FRONTEND_DIR` | none | Built frontend to serve, such as `frontend/dist`. Unset, the server serves only the API |
| `REDIS_URL` | none | A `redis://` or `rediss://` URL to share club details, trainer paces and scans with other instances. Unset, everything stays in this instance's memory. See [Shared cache](#shared-cache) |
| `REDIS_KEY_SECRET` | none | Required with `REDIS_URL`: at least 32 characters, the same on every instance. Keys the hash that replaces trainer IDs in Redis |
| `METRICS_ADDR` | none | IP address and port to serve Prometheus metrics on, such as `127.0.0.1:9464`. Unset, no metrics are recorded. See [Metrics](#metrics) |

Blank values count as unset. These stop the server at startup, with an error message and exit code 1:

- a missing key, or a `HOST` that isn't an IP address (hostnames such as `localhost` aren't accepted)
- a `PORT` outside 1 to 65535, or a `TRUST_FORWARDED_FOR` other than `true` or `false`
- a scan, cache or rate limit setting that isn't a positive whole number, or `SCAN_RANK_FROM` above `SCAN_RANK_TO`
- a `FRONTEND_DIR` without an `index.html`
- a `REDIS_URL` without a `redis://` or `rediss://` scheme, a missing or short `REDIS_KEY_SECRET`, or a Redis that can't be reached within 5 seconds
- a `METRICS_ADDR` that isn't an IP address and port, or that can't be listened on

The API key, `REDIS_URL` (which can hold a password) and `REDIS_KEY_SECRET` are never logged, never included in errors and are redacted in `Debug` output. `.env` is git-ignored; never commit it.

## Scanner

`scan` builds a new [club pool](#club-pool) and fills the [detail cache](#club-details):

1. List clubs by monthly rank, best first, 100 per page and up to `SCAN_RANK_TO`. Stop after a page whose last club is ranked `SCAN_RANK_TO` or worse, after a short or empty page, or after 100 pages.
2. Decide which month to use. Early in a game month the previous month is used instead; see [Early in the month](#early-in-the-month).
3. Build a `ClubPool` from the clubs ranked `SCAN_RANK_FROM` to `SCAN_RANK_TO`. `ListedClub::from_circle` drops archived clubs, full clubs, clubs with fewer than 8 members, and rows missing points, a member count or `last_updated`. Closed clubs are kept, so each request can include or exclude them.
4. Copy in the exact medians of clubs that are still cached for that month, then swap the new pool in.
5. Prefetch member data for up to `SCAN_MAX_DETAILS` clubs in the background lane, so visitors' requests go first. If the pool is larger, the clubs are spread evenly across the estimates, from slowest to fastest. If a club's details come back without club information, the listed details are used.

A club whose details fail to load is logged and skipped. A failed list request fails the whole scan and leaves the previous pool in place. `Scan` returns the number of candidates (the pool size), how many were prefetched, how many failed, and the game month of the listed data.

With the client's default 500 ms rate limit, a default scan takes about 80 seconds: around 9 list pages plus one lookup per candidate, about 150 at the time of writing. Clubs still cached from the previous scan aren't fetched again.

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
| Trainer pace | This month's `daily_fans` | Last month's `daily_fans`, or the 7-day gain ÷ 7 from `rankings_gains` if that month has too little data |
| `month` in responses | Current game month | Previous game month |

The scan decides the month from the list's snapshot time (`is_early_month`), so clubs and trainers always use the same month. The detail cache and the trainer pace cache keep each month's data separately, so the switch on the 4th never mixes months. A detail load that started under the old month doesn't write its median into the new pool.

### Estimate

`ListedClub::from_circle` estimates each club's pace from its list row:

```text
raw rate = monthly_point ÷ elapsed days ÷ member_count
```

The raw rate estimates the club's **mean** daily gain per member. Clubs are left out when they're archived, full, have fewer than 8 members, or are missing points, a member count or a valid `last_updated`, and before a full day of the month has passed. Closed clubs stay in the pool, so each request can include or exclude them.

### Calibration

Fit uses the **median** of current members, which is usually below the mean. `Calibration` corrects for this with one factor for the whole pool:

```text
c = median of (exact median ÷ raw rate)   over clubs with loaded details
estimated median = exact median if known, otherwise c × raw rate
```

`club_median` takes the exact median from the scoring crate's `score_club`, so it is exactly the median the fit formula uses. `ClubPool::set_exact_median` records one for a club, recalculates `c` from every exact median, and re-sorts the pool. `set_exact_medians` does the same for several clubs at once. With fewer than 20 valid samples, `c` is 1.0.

### Shortlist

`ClubPool::shortlist(target, k, include_closed)` returns the `k` clubs whose estimated median is nearest `target`, which is normally `TARGET_MEDIAN_RATIO` (0.85) × your pace. Distance is measured in log space, so half and double the target are equally far:

```text
distance = |ln(estimated median ÷ target)|
```

The pool is sorted once by estimate. A query binary-searches to the target and walks outwards in both directions, skipping closed clubs unless they're included. That costs O(log n + k). Ties go to the lower club ID, and a property test checks that the result always matches sorting every club by distance. Clubs are re-sorted only when exact medians are added, not for each query.

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

Exact medians help most when many clubs have them. With an exact median for every synthetic club, a shortlist of 60 reaches 0.995. On real data, with exact medians for the 80 prefetched clubs (about half the pool), a shortlist of 30 rises from 0.72 to 0.77, and one of 60 barely changes. Fit also depends on where you'd place among the members, so on real data being close in median isn't a strong enough signal on its own.

## Club details

`ClubDetails` is a shared cache of club details (`Club` with members), built on [`moka`](https://crates.io/crates/moka). Clones share one cache.

- **Single flight.** `get(circle_id, priority)` fetches a club only if it isn't cached. Concurrent calls for the same club share one request to uma.moe. Failed fetches aren't cached, so the next call tries again.
- **Expiry.** Entries expire after `DetailConfig::ttl` (180 minutes by default, one scan interval). The cache holds at most `max_clubs` (2,000 by default).
- **Exact medians.** Each successful load writes the club's exact median into the shared `ClubPool`. After a scan builds a new pool, `restore_exact_medians` copies in the medians of clubs that are still cached.
- **Background loading.** `queue(circle_id)` adds a club for `BackgroundLoader::run` to fetch in the background lane. Clubs that are already cached or queued are skipped, and once `queue_size` clubs are waiting (1,000 by default), `queue` returns `false`. A failed background load is logged, and the club leaves the queue.
- **Lanes.** Visitors' fetches use the interactive lane, so they go ahead of queued background loads. If a background load for a club is already running, a visitor asking for that club waits for it rather than starting a second fetch.

`queued()` and `cached_clubs()` report the queue length and the approximate cache size.

## Shared cache

`SharedCache` is a second cache layer for running several server instances against one store. On a local miss, `ClubDetails` and `PaceLookup` check it before calling uma.moe, and store what they load in it with the same expiry, under keys such as `usm:v1:club:<id>:<month>`. A club found there still gives the pool its exact median.

Without `REDIS_URL`, the shared cache stores nothing (`SharedCache::none()`) and each instance works alone. With it, the server uses Redis:

| Situation | What happens |
|---|---|
| Start-up | The server connects and sends `PING`. If Redis can't be reached within 5 seconds, it stops with exit code 1, so a wrong URL is caught straight away |
| Each read or write | Times out after 500 ms. Any error or timeout counts as a miss, and the instance carries on with its own cache and uma.moe |
| Redis goes away | One warning is logged; the connection reconnects by itself, and one message is logged when Redis is back. With [metrics](#metrics) on, `redis_up` shows the state and `redis_errors_total` counts each failure |

**Looked-up trainer IDs never reach Redis.** Pace keys use the first 32 hex characters of HMAC-SHA256(`REDIS_KEY_SECRET`, trainer ID), so every instance needs the same secret. Club keys keep the club ID, which isn't personal, and club data is stored as uma.moe publishes it. Run Redis without persistence (for example `--save "" --appendonly no`), so paces stay in memory as they do without Redis. The `redis` profile in `docker-compose.yml` runs it this way; see [Docker details](../../README.md#docker-details).

Still per instance: rate limits and the background queue.

### Shared scans

With Redis, instances that have the same `SCAN_RANK_FROM`, `SCAN_RANK_TO` and `SCAN_MAX_DETAILS` share one scan, so uma.moe is scanned once per interval however many instances run. Whenever a scan is due, one Redis script decides what the instance does:

| Situation | What the instance does |
|---|---|
| A scan was published less than one interval ago | Loads it unless it already has, then waits until the next scan is due |
| No recent scan, and the scan lock is free | Takes the lock, scans, and publishes the result |
| Another instance holds the lock | Reports `scanning: true` and checks again every second |
| Redis doesn't answer | Scans on its own |

- **Lock.** It lasts 60 seconds and is renewed every 20 seconds while scanning. Shutdown releases it; if an instance crashes, another takes over within 60 seconds.
- **Publishing.** After prefetching, the leader publishes the listed clubs, marks them current for one `SCAN_INTERVAL_MINUTES`, and keeps them for two. The others build their pool from those rows and load club details from Redis, so they barely call uma.moe. An instance that starts later loads the published scan straight away.
- **Failures.** A failed scan is published too: every instance records the same error, keeps its last results, and waits for the next interval.
- **Missing data.** If a scan is marked current but its rows are gone from Redis, the instance scans on its own.

The keys are `usm:v1:scan:<from>-<to>-<max>:lock`, `:fresh` and `:latest`, so instances with different scan windows don't share.

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

Scan results, the club pool and club details live in memory only, so nothing is written to disk. When the server starts it scans straight away, then scans again `SCAN_INTERVAL_MINUTES` after each scan finishes. Until the first scan finishes there are no results; with the defaults that takes about 80 seconds. With Redis, a server that starts while a published scan is current loads it in a second or two instead, and follows that scan's schedule.

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

The policy allows no inline scripts or styles, so the frontend must not add any. That's why the theme script lives in its own file.

## Endpoints

All endpoints are under `/api/v1`.

| Method | Path | Response |
|---|---|---|
| `GET` | `/api/v1/health` | `200 {"status":"ok"}` |
| `GET` | `/api/v1/status` | `200` with the scan state, below |
| `GET` | `/api/v1/recommendations` | Ranked clubs for a trainer or a pace, below |

`/api/v1/status` looks like this:

```json
{
  "scanning": false,
  "last_scan": {
    "finished_at": "2026-03-18T09:00:00Z",
    "clubs": 152,
    "candidates": 152,
    "failed": 0,
    "month": "2026-03"
  },
  "last_error": null,
  "pool": { "clubs": 152, "exact_medians": 152, "calibration": { "factor": 0.86, "samples": 152 } },
  "details": { "cached": 152, "queued": 0 }
}
```

| Field | Meaning |
|---|---|
| `scanning` | Whether a scan is running. With Redis, also while waiting for another instance's scan |
| `last_scan.clubs` | Clubs whose member data the scan prefetched |
| `last_scan.candidates` | Clubs in the pool after the scan's filters |
| `last_scan.month` | Game month of the listed data, `YYYY-MM` |
| `pool.clubs` | Clubs in the current pool |
| `pool.exact_medians` | Pool clubs whose exact median is known from loaded details |
| `pool.calibration` | The mean-to-median factor and how many clubs it's based on |
| `details.cached` | Clubs in the detail cache (approximate) |
| `details.queued` | Clubs waiting for a background load |

`last_scan` is `null` until the first scan succeeds. `last_error` holds `{ "at", "message" }` for a failure since the last success. Times are RFC 3339 in UTC.

`message` is short and safe to show publicly. It never includes request URLs or text from uma.moe's error responses. The full error goes to the server log.

| Message | Cause |
|---|---|
| `listing clubs failed: uma.moe returned 503 Service Unavailable` | uma.moe answered with an error status (the status varies) |
| `listing clubs failed: could not reach uma.moe` | Timeout or connection error |
| `listing clubs failed: unexpected response from uma.moe` | The response wasn't the expected JSON |
| `listing clubs failed: internal error` | A problem in the server itself |

### Recommendations

`GET /api/v1/recommendations` scores every cached club that's in the current pool with [`uma-stable-match-scoring`](../scoring/README.md) and returns them best first.

| Parameter | Required | Default | Meaning |
|---|---|---|---|
| `trainer_id` | One of these two | | Your trainer ID. The server works out your pace from your own fan data this month |
| `pace` | One of these two | | Your daily fan gain, such as `7600000` |
| `balance` | No | `false` | `true` turns on balance scoring at weight 0.5 |
| `balance_weight` | No | | Balance strength, from 0 to 1. Setting it turns balance scoring on |
| `include_closed` | No | `false` | `true` to include clubs that aren't accepting members |
| `limit` | No | `50` | Most clubs to return |

`balance=false` together with `balance_weight` is rejected, because the request contradicts itself.

#### Choosing clubs

1. Check the pace and balance weight. An invalid request fails before anything is fetched.
2. Shortlist the `SHORTLIST_SIZE` clubs whose estimated median is nearest `0.85 ×` your pace. Closed clubs are left out unless `include_closed=true`.
3. Take every cached club in the pool, then fetch up to `REQUEST_FETCH_BUDGET` uncached shortlisted clubs in the interactive lane, nearest first. Requests that need the same club share one fetch.
4. Queue the other uncached shortlisted clubs, and any that failed, for background loading.
5. Score every loaded club in the pool, not only the shortlist.

Cached clubs are always scored exactly, so the shortlist only decides which uncached clubs to load. Once every club is cached, which is the normal case when `SCAN_MAX_DETAILS` is at least the pool size, results are the same as scoring the whole pool. `coverage` says how complete a response is.

#### Looking up a trainer

With `trainer_id`, the server fetches that trainer's club from uma.moe and finds their snapshot (the latest month if there's more than one). It then calculates their pace with `daily_pace`, which needs at least 3 days of data and a gain above zero. [Early in the month](#early-in-the-month), it uses last month's data, falling back to the trainer's 7-day gain.

- **Cached for `TRAINER_CACHE_MINUTES`** (10 by default). A found pace, and a "not found" or "not enough data" answer, is reused for that long, so repeat requests don't call uma.moe again. Failed uma.moe requests aren't cached.
- **IDs stay out of logs.** Trainer IDs never appear in log messages or error responses, only in the response to the trainer's own request. Failed lookups log only the HTTP status.
- **No lookup without results.** Before the first scan finishes, the endpoint returns `503` without contacting uma.moe.

#### Response

Clubs with fewer than 8 rankable members aren't returned, and with balance scoring on, neither are clubs whose balance can't be measured.

```json
{
  "trainer": { "trainer_id": 123456789012, "pace_per_day": 7600000.0 },
  "options": { "balance_weight": 0.5, "include_closed": false, "limit": 50 },
  "updated_at": "2026-03-18T09:00:00Z",
  "month": "2026-03",
  "coverage": { "shortlisted": 102, "scored": 113, "pending": 0 },
  "total": 62,
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
  ]
}
```

| Field | Meaning |
|---|---|
| `trainer.trainer_id` | The trainer ID you asked about, or `null` when you gave a `pace` |
| `trainer.pace_per_day` | The pace used: yours from uma.moe, or the `pace` you gave |
| `options` | The settings used, after defaults: `balance_weight` is `null` when balance scoring is off |
| `updated_at` | When the clubs were scanned |
| `month` | Game month of the club data, `YYYY-MM` |
| `coverage.shortlisted` | Clubs on the shortlist for your pace |
| `coverage.scored` | Clubs with member data that were scored, including cached clubs outside the shortlist |
| `coverage.pending` | Shortlisted clubs still loading in the background. Above 0, searching again shortly can give more results |
| `total` | How many clubs were scored, before `limit` |
| `clubs` | Up to `limit` clubs, best first |

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
| `400` | Any of these:<br>• the query string can't be read, for example because a parameter is repeated<br>• neither or both of `trainer_id` and `pace` are given<br>• `trainer_id` isn't a trainer ID<br>• `pace` isn't a number above zero<br>• `balance_weight` isn't a number from 0 to 1<br>• `balance=false` is given with `balance_weight`<br>• `balance` or `include_closed` isn't `true` or `false`<br>• `limit` isn't a positive whole number |
| `404` | uma.moe doesn't know the trainer, or they aren't in their club's data |
| `422` | The trainer has too little fan data this month to work out a pace |
| `502` | uma.moe couldn't be reached for the trainer lookup |
| `429` | Too many requests from this visitor. The `Retry-After` header says how many seconds to wait |
| `503` | No scan has finished yet. Try again shortly |

### Rate limits

`/api/v1/recommendations` is limited per visitor. Health and status aren't limited.

- **All requests:** `RATE_LIMIT_PER_MINUTE` (30 by default).
- **Trainer lookups:** `TRAINER_LOOKUPS_PER_MINUTE` (10 by default). This only counts `trainer_id` requests that need a call to uma.moe. Cached trainers and `pace` requests don't use it, so a public instance can't be used to flood uma.moe.

Each limit allows a burst of its full size, then frees one request every 60 ÷ limit seconds. With the defaults, that's one request every 2 seconds and one trainer lookup every 6 seconds. Over the limit, the response is `429` with `Retry-After` and an error such as `"too many requests; try again in 2 seconds"`.

Visitors are identified by IP address:

- **IPv6 addresses** are grouped by their first 64 bits, the usual size of one home network.
- **IPv4-mapped IPv6 addresses** count as IPv4.
- **Behind a reverse proxy**, every request comes from the proxy's address. Set `TRUST_FORWARDED_FOR=true` and the server uses the right-most `X-Forwarded-For` address, the one your proxy added. Anything a client writes further left is ignored. Don't turn it on without a proxy, or clients can pick their own address.

IP addresses are kept in memory only while they're being limited, and they're never logged.

## Metrics

With `METRICS_ADDR` set, the server records metrics and serves them in the Prometheus text format at `GET /metrics`, on that address only. The app's port never serves them. Unset, nothing is recorded.

Keep the address private: `127.0.0.1:<port>` on its own. With Docker Compose, set `METRICS_PORT` in `.env` instead: the container listens on `0.0.0.0:9464` and Compose publishes it to `127.0.0.1:<METRICS_PORT>` only. Labels never include trainer IDs or IP addresses. Routes are labelled by their pattern, such as `/api/v1/recommendations`; paths that match no route are labelled `/api (unknown)` or `frontend`.

| Metric | Type | Labels | Meaning |
|---|---|---|---|
| `http_requests_total` | counter | `route`, `method`, `status` | Requests answered, by HTTP status code |
| `http_request_duration_seconds` | histogram | `route`, `method` | Time to answer a request |
| `rate_limited_total` | counter | `limit`: `requests` or `trainer_lookups` | Requests turned away by a [rate limit](#rate-limits) |
| `scan_phase` | gauge | | `0` idle, `1` listing clubs, `2` loading club details |
| `scan_listed_clubs` | gauge | | Clubs in the rank window listed so far in this scan |
| `scan_clubs_to_load`, `scan_clubs_loaded`, `scan_clubs_failed` | gauge | | Progress of the current scan's prefetch |
| `scans_total` | counter | `source`: `alone`, `lead` or `published`; `result`: `ok` or `error` | Scans finished, whether run here or loaded from another instance |
| `scan_duration_seconds` | histogram | `source` | Time from a scan's start to its end |
| `scan_turns_total` | counter | `turn`: `alone`, `lead`, `wait` or `fresh` | What the instance did when a scan was due; see [Shared scans](#shared-scans) |
| `scan_last_success_timestamp_seconds` | gauge | | Unix time of the last successful scan |
| `pool_clubs`, `pool_exact_medians` | gauge | | Clubs in the pool, and how many have an exact median |
| `pool_calibration_factor`, `pool_calibration_samples` | gauge | | The mean-to-median factor and how many clubs it's based on |
| `umamoe_requests_total` | counter | `endpoint`, `lane`, `status` | uma.moe calls by HTTP status code, or `timeout`, `connect_error` or `transport_error`. Each retry counts as another call |
| `umamoe_request_duration_seconds` | histogram | `endpoint`, `lane` | uma.moe response time |
| `umamoe_queue_wait_seconds` | histogram | `lane` | Time waiting for the client's 500 ms spacing |
| `club_detail_lookups_total` | counter | `source`: `memory`, `redis`, `umamoe` or `error` | Where each club's details came from; `error` means the uma.moe fetch failed |
| `pace_lookups_total` | counter | `source`: `memory`, `redis`, `umamoe` or `error` | Where each trainer's pace came from |
| `detail_cache_clubs` | gauge | | Clubs in this instance's detail cache |
| `detail_queue_clubs` | gauge | | Clubs waiting for a background load |
| `redis_up` | gauge | | `1` while Redis answers, `0` after a failure. Absent without Redis |
| `redis_errors_total` | counter | `reason` | Failed or timed-out Redis operations, by the kind of failure |
| `redis_op_duration_seconds` | histogram | `op`: `get`, `set` or `script` | Time per Redis operation, failures included |

`scan_last_success_timestamp_seconds` and the `pool_*` and `detail_*` values are read from the server each time `/metrics` is requested; the detail cache settles its pending work first, so `detail_cache_clubs` is exact. Histograms use buckets from 1 ms to 1 hour. The `umamoe_*` metrics come from the uma.moe client; its [README](../umamoe-client/README.md#metrics) describes them in full.

To watch these on a dashboard, [observability/README.md](../../observability/README.md) sets up Prometheus and Grafana locally, with a ready-made dashboard.

## Shutdown

Ctrl+C or SIGTERM (as sent by `docker stop`) stops accepting new connections, including on the metrics address, and cancels any running scan. In-flight requests finish, and then the server exits.
