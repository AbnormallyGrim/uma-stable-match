# uma-stable-match-server

The API server for Uma Stable Match. It scans uma.moe for candidate clubs on a schedule, keeps the results in memory, and recommends clubs for a trainer ID or a daily fan pace.

## Run

```bash
cp .env.example .env    # then add your uma.moe API key after UMAMOE_API_KEY=
cargo run -p uma-stable-match-server
```

Run these from the repository root. The server reads the first `.env` it finds in the directory it's started in or a parent directory. Real environment variables take precedence over `.env`.

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
| `SCAN_MAX_DETAILS` | `80` | Most clubs to fetch member data for in each scan |
| `SCAN_INTERVAL_MINUTES` | `180` | Minutes between scans. The default may change once uma.moe agrees how often scans can run |
| `TRAINER_CACHE_MINUTES` | `10` | Minutes a trainer's looked-up pace is reused |

Blank values count as unset. A missing key, a `HOST` that isn't an IP address (hostnames such as `localhost` aren't accepted), a `PORT` outside 1 to 65535, a scan or cache setting that isn't a positive whole number, or `SCAN_RANK_FROM` above `SCAN_RANK_TO` stops the server at startup, with an error message and exit code 1. The key is never logged, never included in errors and is redacted in `Debug` output. `.env` is git-ignored; never commit it.

## Scanner

`scan` finds clubs worth recommending and fetches their member data, following the same rules as the original prototype:

1. List clubs by monthly rank, best first, 100 per page and up to `SCAN_RANK_TO`. Stop after a page whose last club is ranked `SCAN_RANK_TO` or worse, after a short or empty page, or after 100 pages.
2. Keep clubs ranked from `SCAN_RANK_FROM` to `SCAN_RANK_TO` that have open spots. A club with an unknown or zero member count is treated as full.
3. Sort climbers first, by last month's rank minus this month's. A club with no rank last month counts as a big climber. Clubs that climbed equally keep their list order.
4. Fetch member data for the first `SCAN_MAX_DETAILS` clubs and convert each one to a `Club`. If a club's details come back without club information, the listed details are used.

A club whose details fail to load is logged and skipped. A failed list request fails the whole scan. `Scan` returns the loaded clubs, the number of candidates and the number that failed.

With the client's default 500 ms rate limit, a default scan takes about 45 seconds: around 9 list pages plus 80 club lookups.

## Cache and schedule

Scan results live in memory only, so nothing is written to disk. When the server starts it scans straight away, then scans again `SCAN_INTERVAL_MINUTES` after each scan finishes. Until the first scan finishes there are no results; with the defaults that takes about 45 seconds.

- **No overlapping scans.** A new scan won't start while one is running.
- **Failures keep the last results.** A failed scan is logged and recorded as the last error, and the previous results stay in place. The next successful scan replaces the results and clears the error.
- **Shutdown cancels a running scan.** The cache then keeps whatever it had before that scan started.

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
  "last_scan": { "finished_at": "2026-03-18T09:00:00Z", "clubs": 80, "candidates": 214, "failed": 0 },
  "last_error": null
}
```

`last_scan` is `null` until the first scan succeeds. `last_error` holds `{ "at", "message" }` for a failure since the last success. Times are RFC 3339 in UTC.

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

- `join_style` is `open`, `approval`, `closed` or `null`.
- `tier` runs from `D` to `SS`, or is `null` when unknown.
- `policy` is uma.moe's playstyle code.
- `balance` is `null` unless balance scoring is on.
- Lower `fit` and `score` are better.

The [scoring README](../scoring/README.md) explains every value.

#### Errors

Errors come back as `{"error": "..."}`:

| Status | When |
|---|---|
| `400` | Any of these:<br>• neither or both of `trainer_id` and `pace` are given<br>• `trainer_id` isn't a trainer ID<br>• `pace` isn't a number above zero<br>• `balance_weight` isn't a number from 0 to 1<br>• `balance=false` is given with `balance_weight`<br>• `balance` or `include_closed` isn't `true` or `false`<br>• `limit` isn't a positive whole number |
| `404` | uma.moe doesn't know the trainer, or they aren't in their club's data |
| `422` | The trainer has too little fan data this month to work out a pace |
| `502` | uma.moe couldn't be reached for the trainer lookup |
| `503` | No scan has finished yet. Try again shortly |

## Shutdown

Ctrl+C or SIGTERM (as sent by `docker stop`) stops accepting new connections and cancels any running scan. In-flight requests finish, and then the server exits.
