# uma-stable-match-server

The API server for Uma Stable Match. It scans uma.moe for candidate clubs on a schedule, keeps the results in memory, and recommends clubs for a trainer ID or a daily fan pace.

## Run

```bash
cp .env.example .env    # then add your uma.moe API key after UMAMOE_API_KEY=
cargo run -p uma-stable-match-server
```

Run these from the repository root. The server reads the first `.env` it finds in the directory it's started in or a parent directory. Real environment variables take precedence over `.env`.

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
| `SCAN_MAX_DETAILS` | `200` | Most clubs to fetch member data for in each scan |
| `SCAN_INTERVAL_MINUTES` | `180` | Minutes between scans. The default may change once uma.moe agrees how often scans can run |
| `TRAINER_CACHE_MINUTES` | `10` | Minutes a trainer's looked-up pace is reused |
| `RATE_LIMIT_PER_MINUTE` | `30` | Recommendation requests each visitor can make per minute |
| `TRAINER_LOOKUPS_PER_MINUTE` | `10` | Trainer lookups each visitor can make per minute that need a call to uma.moe |
| `TRUST_FORWARDED_FOR` | `false` | `true` to identify visitors by `X-Forwarded-For`. Only turn this on behind a reverse proxy that sets the header |
| `FRONTEND_DIR` | none | Built frontend to serve, such as `frontend/dist`. Unset, the server serves only the API |

Blank values count as unset. A missing key, a `HOST` that isn't an IP address (hostnames such as `localhost` aren't accepted), a `PORT` outside 1 to 65535, a `TRUST_FORWARDED_FOR` other than `true` or `false`, a scan, cache or rate limit setting that isn't a positive whole number, `SCAN_RANK_FROM` above `SCAN_RANK_TO`, or a `FRONTEND_DIR` without an `index.html` stops the server at startup, with an error message and exit code 1. The key is never logged, never included in errors and is redacted in `Debug` output. `.env` is git-ignored; never commit it.

## Scanner

`scan` finds clubs worth recommending and fetches their member data:

1. List clubs by monthly rank, best first, 100 per page and up to `SCAN_RANK_TO`. Stop after a page whose last club is ranked `SCAN_RANK_TO` or worse, after a short or empty page, or after 100 pages.
2. Keep clubs ranked from `SCAN_RANK_FROM` to `SCAN_RANK_TO` that aren't archived and have 8 to 29 members. Full clubs have no room, smaller ones can't be scored, and a club with an unknown member count is skipped. Closed clubs are kept, so each request can include or exclude them.
3. Fetch member data for the first `SCAN_MAX_DETAILS` clubs, best rank first, and convert each one to a `Club`. If there are more candidates, a warning is logged. If a club's details come back without club information, the listed details are used.

A club whose details fail to load is logged and skipped. A failed list request fails the whole scan. `Scan` returns the loaded clubs, the number of candidates and the number that failed.

With the client's default 500 ms rate limit, a default scan takes about 80 seconds: around 9 list pages plus one lookup per candidate, about 150 at the time of writing.

## Club pool

The club pool is the first stage of two-stage matching (#87). It picks the clubs worth scoring for one trainer, using only the list endpoint's data. It isn't used by the scanner or recommendations yet.

### Game months

uma.moe's game month starts on the **2nd at 00:00 JST** (UTC+09:00), so the 1st still belongs to the previous month. `game_month_start` and `elapsed_days` follow this. A list row's `last_updated` is the daily 00:00 JST snapshot, so `elapsed_days` at that time is a whole number of days.

### Estimate

`ListedClub::from_circle` estimates each club's pace from its list row:

```text
raw rate = monthly_point ÷ elapsed days ÷ member_count
```

The raw rate estimates the club's **mean** daily gain per member. Clubs are left out when they're archived, full, have fewer than 8 members, or are missing points, a member count or a valid `last_updated`, and before a full day of the month has passed. Closed clubs stay in the pool, so each request can include or exclude them.

### Calibration

Fit uses the **median** of current members, which is usually below the mean. `Calibration` corrects for this with one factor for the whole pool:

```text
c = median of (true median ÷ raw rate)   over clubs with loaded details
estimated median = c × raw rate
```

`Calibration::ratio` takes the true median from the scoring crate's `score_club`, so it is exactly the median the fit formula uses. With fewer than 20 valid samples, `c` is 1.0.

### Shortlist

`ClubPool::shortlist(target, k, include_closed)` returns the `k` clubs whose estimated median is nearest `target`, which is normally `TARGET_MEDIAN_RATIO` (0.85) × your pace. Distance is measured in log space, so half and double the target are equally far:

```text
distance = |ln(estimated median ÷ target)|
```

The pool is sorted once by estimate. A query binary-searches to the target and walks outwards in both directions, skipping closed clubs unless they're included. That costs O(log n + k). Ties go to the lower club ID, and a property test checks that the result always matches sorting every club by distance. A new calibration only moves the target, so it never re-sorts the pool.

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

## Cache and schedule

Scan results live in memory only, so nothing is written to disk. When the server starts it scans straight away, then scans again `SCAN_INTERVAL_MINUTES` after each scan finishes. Until the first scan finishes there are no results; with the defaults that takes about 45 seconds.

- **No overlapping scans.** A new scan won't start while one is running.
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
  "last_scan": { "finished_at": "2026-03-18T09:00:00Z", "clubs": 152, "candidates": 152, "failed": 0 },
  "last_error": null
}
```

`last_scan` is `null` until the first scan succeeds. `last_error` holds `{ "at", "message" }` for a failure since the last success. Times are RFC 3339 in UTC.

`message` is short and safe to show publicly. It never includes request URLs or text from uma.moe's error responses. The full error goes to the server log.

| Message | Cause |
|---|---|
| `listing clubs failed: uma.moe returned 503 Service Unavailable` | uma.moe answered with an error status (the status varies) |
| `listing clubs failed: could not reach uma.moe` | Timeout or connection error |
| `listing clubs failed: unexpected response from uma.moe` | The response wasn't the expected JSON |
| `listing clubs failed: internal error` | A problem in the server itself |

### Recommendations

`GET /api/v1/recommendations` scores the clubs from the last scan with [`uma-stable-match-scoring`](../scoring/README.md) and returns them best first.

| Parameter | Required | Default | Meaning |
|---|---|---|---|
| `trainer_id` | One of these two | | Your trainer ID. The server works out your pace from your own fan data this month |
| `pace` | One of these two | | Your daily fan gain, such as `7600000` |
| `balance` | No | `false` | `true` turns on balance scoring at weight 0.5 |
| `balance_weight` | No | | Balance strength, from 0 to 1. Setting it turns balance scoring on |
| `include_closed` | No | `false` | `true` to include clubs that aren't accepting members |
| `limit` | No | `50` | Most clubs to return |

`balance=false` together with `balance_weight` is rejected, because the request contradicts itself.

#### Looking up a trainer

With `trainer_id`, the server fetches that trainer's club from uma.moe and finds their snapshot (the latest month if there's more than one). It then calculates their pace with `daily_pace`, which needs at least 3 days of data and a gain above zero.

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

## Shutdown

Ctrl+C or SIGTERM (as sent by `docker stop`) stops accepting new connections and cancels any running scan. In-flight requests finish, and then the server exits.
